#[macro_use]
extern crate bitfield;

mod registers;

use registers::*;

use std::cell::RefCell;
use std::rc::Rc;

use memory::Memory;

pub struct PPU {
    vram: Rc<RefCell<dyn Memory>>,
    palette_ram: Rc<RefCell<dyn Memory>>,
    oam: Rc<RefCell<dyn Memory>>,

    scan_line: u8,
    scan_cycle: u32,

    lcd_control_reg: LcdControlReg,
    bg_control_regs: [BgControlReg; 4],

    framebuffer: [u8; 240 * 160 * 2],
    frame_ready: bool,
}

impl PPU {
    pub fn new(
        vram: Rc<RefCell<dyn Memory>>,
        palette_ram: Rc<RefCell<dyn Memory>>,
        oam: Rc<RefCell<dyn Memory>>,
    ) -> Self {
        Self {
            vram,
            palette_ram,
            oam,

            scan_line: 0,
            scan_cycle: 0,

            lcd_control_reg: LcdControlReg(0),
            bg_control_regs: [
                BgControlReg(0),
                BgControlReg(0),
                BgControlReg(0),
                BgControlReg(0),
            ],

            framebuffer: [0; 240 * 160 * 2],
            frame_ready: false,
        }
    }

    pub fn tick(&mut self) {
        if self.scan_cycle == 0 && self.scan_line < 160 {
            self.draw_scanline();
        }
        self.increment_scan();
    }

    pub fn try_get_framebuffer(&mut self) -> Option<[u8; 240 * 160 * 2]> {
        let temp = self.frame_ready;
        self.frame_ready = false;
        temp.then(|| self.framebuffer)
    }

    pub fn vcount(&self) -> u8 {
        self.scan_line
    }

    fn increment_scan(&mut self) {
        self.scan_cycle = (self.scan_cycle + 1) % 1232;

        if self.scan_cycle == 0 {
            self.scan_line = (self.scan_line + 1) % 228;

            if self.scan_line == 160 {
                self.frame_ready = true;
                // TODO: VBlank interrupt if enabled
            }

            // TODO: V-Counter interrupt if enabled
        } else if self.scan_cycle == 960 {
            // TODO: HBlank interrupt if enabled
        }
    }

    fn draw_scanline(&mut self) {
        // TODO: Should generate a line from each active layer (backgrounds, obj),
        // taking into account windowing, then combine them based on priority and blending
        // TODO: Lots of this logic can be abstracted and modularized
        match self.lcd_control_reg.bg_mode() {
            0 => {
                // TODO: Scroll
                let map_base = self.bg_control_regs[0].screen_block() as usize * 0x800;
                let tile_row = self.scan_line as usize / 8;
                for tile_col in 0..30 {
                    let map_entry = self
                        .vram
                        .borrow_mut()
                        .read_u16(map_base + 2 * (tile_row * 32 + tile_col));
                    let tile_n = map_entry & 0b11_1111_1111;
                    let flip_h = (map_entry >> 10) & 1 == 1;
                    let flip_v = (map_entry >> 11) & 1 == 1;
                    let palette_n = (map_entry >> 12) & 0b1111;
                    let row = self.scan_line as usize % 8;
                    for byte_n in 0..4 {
                        let data = self.vram.borrow_mut().read(
                            0x4000 * self.bg_control_regs[0].char_block() as usize
                                + 32 * tile_n as usize
                                + row * 4
                                + byte_n,
                        );
                        let color_i_left = data & 0b1111;
                        let color_left = self
                            .palette_ram
                            .borrow_mut()
                            .read_u16(2 * (palette_n as usize * 16 + color_i_left as usize));
                        let color_i_right = (data >> 4) & 0b1111;
                        let color_right = self
                            .palette_ram
                            .borrow_mut()
                            .read_u16(2 * (palette_n as usize * 16 + color_i_right as usize));
                        self.framebuffer
                            [480 * (8 * tile_row + row) + 16 * tile_col + 4 * byte_n + 0] =
                            color_left as u8;
                        self.framebuffer
                            [480 * (8 * tile_row + row) + 16 * tile_col + 4 * byte_n + 1] =
                            (color_left >> 8) as u8;
                        self.framebuffer
                            [480 * (8 * tile_row + row) + 16 * tile_col + 4 * byte_n + 2] =
                            color_right as u8;
                        self.framebuffer
                            [480 * (8 * tile_row + row) + 16 * tile_col + 4 * byte_n + 3] =
                            (color_right >> 8) as u8;
                    }
                }
            }
            1 => {}
            2 => {}
            3 => {
                let y = self.scan_line as usize / 8;
                let mut pixel_i = 2 * y;
                for x in 0..240 {
                    let color = self.vram.borrow_mut().read_u16(2 * (x + 240 * y));
                    self.framebuffer[pixel_i + 0] = color as u8;
                    self.framebuffer[pixel_i + 1] = (color >> 8) as u8;
                    pixel_i += 2;
                }
            }
            4 => {
                let y = self.scan_line as usize;
                let mut pixel_i = 480 * y;
                for x in 0..240 {
                    let color_i = self.vram.borrow_mut().read(x + 240 * y);
                    let color = self
                        .palette_ram
                        .borrow_mut()
                        .read_u16(2 * (color_i as usize));
                    self.framebuffer[pixel_i + 0] = color as u8;
                    self.framebuffer[pixel_i + 1] = (color >> 8) as u8;
                    pixel_i += 2;
                }
            }
            5 => {}
            _ => panic!("illegal video mode"),
        }
    }
}

impl Memory for PPU {
    fn peek(&self, addr: usize) -> u8 {
        // TODO
        match addr {
            0x004 => {
                (((self.scan_cycle >= 160) as u8) << 1)
                    | (160..=226).contains(&self.scan_line) as u8
            }
            // TODO: 005 is vcount setting
            0x006 => self.scan_line,
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        // TODO
        match addr {
            0x000 => self.lcd_control_reg.set_lo_byte(data),
            0x001 => self.lcd_control_reg.set_hi_byte(data),
            0x008 => self.bg_control_regs[0].set_lo_byte(data),
            0x009 => self.bg_control_regs[0].set_hi_byte(data),
            0x00A => self.bg_control_regs[1].set_lo_byte(data),
            0x00B => self.bg_control_regs[1].set_hi_byte(data),
            0x00C => self.bg_control_regs[2].set_lo_byte(data),
            0x00D => self.bg_control_regs[2].set_hi_byte(data),
            0x00E => self.bg_control_regs[3].set_lo_byte(data),
            0x00F => self.bg_control_regs[3].set_hi_byte(data),
            _ => {}
        }
    }
}
