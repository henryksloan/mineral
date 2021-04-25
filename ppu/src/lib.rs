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
    scroll_regs: [(ScrollReg, ScrollReg); 4],

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
            scroll_regs: [
                (ScrollReg(0), ScrollReg(0)),
                (ScrollReg(0), ScrollReg(0)),
                (ScrollReg(0), ScrollReg(0)),
                (ScrollReg(0), ScrollReg(0)),
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
                let bgs_enabled = [
                    self.lcd_control_reg.enable_bg0(),
                    self.lcd_control_reg.enable_bg1(),
                    self.lcd_control_reg.enable_bg2(),
                    self.lcd_control_reg.enable_bg3(),
                ];
                // Enabled scanlines tupled with the two sorting parameters (priority and bg index)
                let mut lines = (0..4)
                    .filter(|&i| bgs_enabled[i])
                    .map(|i| {
                        (
                            (self.bg_control_regs[i].priority(), i),
                            self.get_text_bg_scanline(i),
                        )
                    })
                    .collect::<Vec<((u16, usize), [u8; 480])>>();

                if lines.len() == 0 {
                    return;
                }

                lines.sort_by_key(|tuple| tuple.0);

                let first_pixel_i = 480 * self.scan_line as usize;
                for i in 0..240 {
                    let mut color = (0, 0);
                    for line in &lines {
                        if line.1[i * 2] != 0 || line.1[i * 2 + 1] != 0 {
                            color.0 = line.1[i * 2];
                            color.1 = line.1[i * 2 + 1];
                            break;
                        }
                    }
                    let pixel_i = first_pixel_i + 2 * i;
                    self.framebuffer[pixel_i + 0] = color.0;
                    self.framebuffer[pixel_i + 1] = color.1;
                }
            }
            1 => {}
            2 => {}
            3 => {
                let y = self.scan_line as usize;
                let mut pixel_i = 480 * y;
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
                let page_base = if self.lcd_control_reg.frame_select() {
                    0x9600
                } else {
                    0
                };
                for x in 0..240 {
                    let color_i = self.vram.borrow_mut().read(page_base + x + 240 * y);
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

    fn get_text_bg_scanline(&self, bg_n: usize) -> [u8; 480] {
        let mut out = [0; 480];

        let (n_bg_cols, n_bg_rows) = match self.bg_control_regs[bg_n].size() {
            0b00 => (32, 32),
            0b01 => (64, 32),
            0b10 => (32, 64),
            0b11 | _ => (64, 64),
        };
        let full_palette_mode = self.bg_control_regs[bg_n].colors();

        let offset_x = (self.scroll_regs[bg_n].0.offset() as usize) % (n_bg_cols * 8);
        let offset_y = (self.scroll_regs[bg_n].1.offset() as usize) % (n_bg_rows * 8);

        let map_base = self.bg_control_regs[bg_n].screen_block() as usize * 0x800;
        let tile_row = ((self.scan_line as usize + offset_y) / 8) % n_bg_rows;
        let first_tile_col = (offset_x / 8) % n_bg_cols;

        let row = (self.scan_line as usize + offset_y) % 8;

        for tile_col in first_tile_col..(first_tile_col + 31) {
            let screen_offset_x = if 31 < tile_col && tile_col < 64 && n_bg_cols == 64 {
                0x800
            } else {
                0
            };
            let screen_offset_y = if 31 < tile_row && tile_row < 64 && n_bg_rows == 64 {
                (n_bg_cols / 32) * 0x800
            } else {
                0
            };
            let map_entry = self.vram.borrow_mut().read_u16(
                map_base
                    + screen_offset_x
                    + screen_offset_y
                    + 2 * (tile_row * 32 + (tile_col % 32)),
            );
            let tile_n = map_entry & 0b11_1111_1111;
            let flip_h = (map_entry >> 10) & 1 == 1;
            let flip_v = (map_entry >> 11) & 1 == 1;
            let palette_n = (map_entry >> 12) & 0b1111;
            if full_palette_mode {
                for byte_n in 0..8 {
                    let pixel_x_offset = ((tile_col - first_tile_col) * 8 + byte_n) as isize
                        - (offset_x as isize) % 8;
                    let data = self.vram.borrow_mut().read(
                        0x4000 * self.bg_control_regs[bg_n].char_block() as usize
                            + 64 * tile_n as usize
                            + (if flip_v { 7 - row } else { row }) * 8
                            + (if flip_h { 3 - byte_n } else { byte_n }) * 2,
                    );
                    let color = self.palette_ram.borrow_mut().read_u16(data as usize);
                    if pixel_x_offset >= 0 && pixel_x_offset <= 239 {
                        out[2 * (pixel_x_offset as usize) + 0] = color as u8;
                        out[2 * (pixel_x_offset as usize) + 1] = (color >> 8) as u8;
                    }
                }
            } else {
                for byte_n in 0..4 {
                    let pixel_x_offset = ((tile_col - first_tile_col) * 8 + 2 * byte_n) as isize
                        - (offset_x as isize) % 8;
                    let data = self.vram.borrow_mut().read(
                        0x4000 * self.bg_control_regs[bg_n].char_block() as usize
                            + 32 * tile_n as usize
                            + (if flip_v { 7 - row } else { row }) * 4
                            + (if flip_h { 3 - byte_n } else { byte_n }),
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
                    if pixel_x_offset >= 0 && pixel_x_offset <= 239 {
                        out[2 * (pixel_x_offset as usize) + 0] = color_left as u8;
                        out[2 * (pixel_x_offset as usize) + 1] = (color_left >> 8) as u8;
                    }
                    if pixel_x_offset >= -1 && (pixel_x_offset + 1) <= 239 {
                        out[(2 * pixel_x_offset + 2) as usize] = color_right as u8;
                        out[(2 * pixel_x_offset + 3) as usize] = (color_right >> 8) as u8;
                    }
                }
            }
        }

        out
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

            0x010 => self.scroll_regs[0].0.set_lo_byte(data),
            0x011 => self.scroll_regs[0].0.set_hi_byte(data),
            0x012 => self.scroll_regs[0].1.set_lo_byte(data),
            0x013 => self.scroll_regs[0].1.set_hi_byte(data),
            0x014 => self.scroll_regs[1].0.set_lo_byte(data),
            0x015 => self.scroll_regs[1].0.set_hi_byte(data),
            0x016 => self.scroll_regs[1].1.set_lo_byte(data),
            0x017 => self.scroll_regs[1].1.set_hi_byte(data),
            0x018 => self.scroll_regs[2].0.set_hi_byte(data),
            0x019 => self.scroll_regs[2].0.set_hi_byte(data),
            0x01A => self.scroll_regs[2].1.set_hi_byte(data),
            0x01B => self.scroll_regs[2].1.set_hi_byte(data),
            0x01C => self.scroll_regs[3].0.set_hi_byte(data),
            0x01D => self.scroll_regs[3].0.set_hi_byte(data),
            0x01E => self.scroll_regs[3].1.set_hi_byte(data),
            0x01F => self.scroll_regs[3].1.set_hi_byte(data),
            _ => {}
        }
    }
}
