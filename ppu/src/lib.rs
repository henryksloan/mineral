#[macro_use]
extern crate bitfield;

mod obj_attrs;
mod registers;

use obj_attrs::*;
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
    lcd_status_reg: LcdStatusReg,
    bg_control_regs: [BgControlReg; 4],
    scroll_regs: [(ScrollReg, ScrollReg); 4],
    win0_coords: (WinCoordReg, WinCoordReg),
    win1_coords: (WinCoordReg, WinCoordReg),
    win_inside: WinInsideControlReg,
    win_outside: WinOutsideControlReg,

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
            lcd_status_reg: LcdStatusReg(0),
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
            win0_coords: (WinCoordReg(0), WinCoordReg(0)),
            win1_coords: (WinCoordReg(0), WinCoordReg(0)),
            win_inside: WinInsideControlReg(0),
            win_outside: WinOutsideControlReg(0),

            framebuffer: [0; 240 * 160 * 2],
            frame_ready: false,
        }
    }

    // Returns (vblank, hblank, vblank_irq, hblank_irq, vcounter_irq)
    pub fn tick(&mut self) -> (bool, bool, bool, bool, bool) {
        if self.scan_cycle == 0 && self.scan_line < 160 {
            self.draw_scanline();
        }
        self.increment_scan()
    }

    pub fn try_get_framebuffer(&mut self) -> Option<[u8; 240 * 160 * 2]> {
        let temp = self.frame_ready;
        self.frame_ready = false;
        temp.then(|| self.framebuffer)
    }

    pub fn vcount(&self) -> u8 {
        self.scan_line
    }

    fn increment_scan(&mut self) -> (bool, bool, bool, bool, bool) {
        let mut vblank = false;
        let mut hblank = false;
        let mut vblank_irq = false;
        let mut hblank_irq = false;
        let mut vcounter_irq = false;

        self.scan_cycle = (self.scan_cycle + 1) % 1232;

        if self.scan_cycle == 0 {
            self.scan_line = (self.scan_line + 1) % 228;

            if self.scan_line == 160 {
                self.frame_ready = true;
                vblank = true;
                if self.lcd_status_reg.vblank_irq() {
                    vblank_irq = true;
                }
                self.draw_sprites();
            }

            if self.scan_line == self.lcd_status_reg.vcounter_line() {
                if self.lcd_status_reg.vcounter_irq() {
                    vcounter_irq = true;
                }
            }
        } else if self.scan_cycle == 960 && self.scan_line < 160 {
            hblank = true;
            if self.lcd_status_reg.hblank_irq() {
                hblank_irq = true;
            }
        }

        self.lcd_status_reg
            .set_vblank((160..=226).contains(&self.scan_line));
        // self.lcd_status_reg
        //     .set_hblank((0..=227).contains(&self.scan_cycle));
        self.lcd_status_reg.set_hblank(self.scan_cycle >= 960);
        self.lcd_status_reg
            .set_vcounter(self.scan_line == self.lcd_status_reg.vcounter_line());

        (vblank, hblank, vblank_irq, hblank_irq, vcounter_irq)
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
                    .collect::<Vec<((u16, usize), [Option<(u8, u8)>; 240])>>();

                lines.sort_by_key(|tuple| tuple.0);

                let first_pixel_i = 480 * self.scan_line as usize;
                for i in 0..240 {
                    let mut found_pixel = false;
                    let mut color = (0, 0);
                    for line in &lines {
                        if let Some(pixel_color) = line.1[i] {
                            color.0 = pixel_color.0;
                            color.1 = pixel_color.1;
                            found_pixel = true;
                            break;
                        }
                    }

                    // TODO: Investigate transparency and background color in other modes
                    let pixel_i = first_pixel_i + 2 * i;
                    if found_pixel {
                        self.framebuffer[pixel_i + 0] = color.0;
                        self.framebuffer[pixel_i + 1] = color.1;
                    } else {
                        let color = self.palette_ram.borrow_mut().read_u16(0);
                        self.framebuffer[pixel_i + 0] = (color & 0xFF) as u8;
                        self.framebuffer[pixel_i + 1] = (color >> 8) as u8;
                    }
                }
            }
            1 => {
                println!("Mode 1");
            }
            2 => {
                println!("Mode 2");
            }
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
                    0xA000
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
            5 => {
                let y = self.scan_line as usize;
                let mut pixel_i = 480 * y;
                let page_base = if self.lcd_control_reg.frame_select() {
                    0xA000
                } else {
                    0
                };
                for x in 0..240 {
                    let color = if y >= 128 || x >= 160 {
                        0
                    } else {
                        self.vram
                            .borrow_mut()
                            .read_u16(page_base + 2 * (x + 160 * y))
                    };
                    self.framebuffer[pixel_i + 0] = color as u8;
                    self.framebuffer[pixel_i + 1] = (color >> 8) as u8;
                    pixel_i += 2;
                }
            }
            _ => panic!("illegal video mode"),
        }
    }

    fn get_text_bg_scanline(&self, bg_n: usize) -> [Option<(u8, u8)>; 240] {
        let mut out = [None; 240];

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
                            + (if flip_h { 7 - byte_n } else { byte_n }),
                    );
                    let color = self.palette_ram.borrow_mut().read_u16(2 * data as usize);
                    let visible_with_windows = self.pixel_visible_with_windows(
                        bg_n,
                        pixel_x_offset as u16,
                        self.scan_line as u16,
                    );
                    if visible_with_windows && pixel_x_offset >= 0 && pixel_x_offset <= 239 {
                        out[pixel_x_offset as usize] = Some((color as u8, (color >> 8) as u8));
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
                    let color_i_right = (data >> 4) & 0b1111;
                    let (color_i_left, color_i_right) = if flip_h {
                        (color_i_right, color_i_left)
                    } else {
                        (color_i_left, color_i_right)
                    };
                    let color_left = self
                        .palette_ram
                        .borrow_mut()
                        .read_u16(2 * (palette_n as usize * 16 + color_i_left as usize));
                    let color_right = self
                        .palette_ram
                        .borrow_mut()
                        .read_u16(2 * (palette_n as usize * 16 + color_i_right as usize));
                    let visible_with_windows = self.pixel_visible_with_windows(
                        bg_n,
                        pixel_x_offset as u16,
                        self.scan_line as u16,
                    );
                    if visible_with_windows && pixel_x_offset >= 0 && pixel_x_offset <= 239 {
                        if color_i_left != 0 {
                            out[pixel_x_offset as usize] =
                                Some((color_left as u8, (color_left >> 8) as u8));
                        }
                    }
                    let visible_with_windows = self.pixel_visible_with_windows(
                        bg_n,
                        (pixel_x_offset as u16).wrapping_add(1),
                        self.scan_line as u16,
                    );
                    if visible_with_windows && pixel_x_offset >= -1 && (pixel_x_offset + 1) <= 239 {
                        if color_i_right != 0 {
                            out[(pixel_x_offset + 1) as usize] =
                                Some((color_right as u8, (color_right >> 8) as u8));
                        }
                    }
                }
            }
        }

        out
    }

    // Checks whether the given pixel coordinate is visible within the enabled windows.
    // bg_n == 4 corresponds to the OBJ layer
    fn pixel_visible_with_windows(&self, bg_n: usize, x: u16, y: u16) -> bool {
        let enabled_windows = {
            let mut windows = vec![];
            if self.lcd_control_reg.enable_win0() {
                windows.push(0);
            }
            if self.lcd_control_reg.enable_win1() {
                windows.push(1);
            }
            windows
        };

        if enabled_windows.is_empty() {
            return true;
        }

        let mut inside_windows = enabled_windows.into_iter().map(|window| {
            let coords = if window == 0 {
                &self.win0_coords
            } else {
                &self.win1_coords
            };

            // TODO: Properly handle off-screen values
            let x_range = {
                let lo = coords.0.coord_lo();
                let mut hi = coords.0.coord_hi();
                if hi > 240 || lo > hi {
                    hi = 240;
                }
                lo..hi
            };
            let y_range = {
                let lo = coords.1.coord_lo();
                let mut hi = coords.1.coord_hi();
                if hi > 160 || lo > hi {
                    hi = 160;
                }
                lo..hi
            };

            x_range.contains(&x) && y_range.contains(&y)
        });

        let visible = match inside_windows.position(|inside| inside == true) {
            Some(window) => {
                // The pixel is inside some window;
                // Check whether this BG is enabled within the highest priority window
                let enabled_inside = (self.win_inside.0 >> (8 * window + bg_n)) & 1 == 1;
                enabled_inside
            }
            None => {
                // The pixel is outside all enabled windows;
                // Check whether this BG is enabled in the outside window
                let enabled_outside = (self.win_outside.0 >> bg_n) & 1 == 1;
                enabled_outside
            }
        };

        visible
    }

    fn draw_sprites(&mut self) {
        let priority_order = {
            let mut sprite_nums = (0..128).rev().collect::<Vec<_>>();
            sprite_nums.sort_by_key(|sprite_n| {
                let oam_addr = 8 * sprite_n;
                let attr2 = ObjAttr2(self.oam.borrow_mut().read_u16(oam_addr + 4));
                3 - attr2.priority()
            });
            sprite_nums
        };
        for sprite_n in priority_order.into_iter() {
            let oam_addr = 8 * sprite_n;
            let attr0 = ObjAttr0(self.oam.borrow_mut().read_u16(oam_addr + 0));
            let attr1 = ObjAttr1(self.oam.borrow_mut().read_u16(oam_addr + 2));
            let attr2 = ObjAttr2(self.oam.borrow_mut().read_u16(oam_addr + 4));

            let bytes_per_tile = if attr0.colors() { 2 } else { 1 };

            if !attr0.affine() && attr0.disable() {
                continue;
            }

            let size = {
                let size_map = match attr0.shape() {
                    0 => [(1, 1), (2, 2), (4, 4), (8, 8)], // Square
                    1 => [(2, 1), (4, 1), (4, 2), (8, 4)], // Horizontal
                    2 => [(1, 2), (1, 4), (2, 4), (4, 8)], // Vertical
                    _ => continue,                         // panic!("Illegal obj shape"),
                };
                size_map[attr1.size() as usize]
            };

            for row in 0..size.1 {
                let y = attr0.y_coord() + 8 * row;
                let row_start = attr2.tile()
                    + (if attr1.flip_v() {
                        (size.1 - 1) - row
                    } else {
                        row
                    }) * if self.lcd_control_reg.obj_char_mapping() {
                        size.0 * bytes_per_tile // 1D
                    } else {
                        0x20 // 2D
                    };

                let background_color = self.palette_ram.borrow_mut().read_u16(0);
                for col in 0..size.0 {
                    let tile_n = row_start
                        + (if attr1.flip_h() {
                            (size.0 - 1) - col
                        } else {
                            col
                        }) * bytes_per_tile;
                    let x = attr1.x_coord() + 8 * col;
                    for pixel_y in 0..8 {
                        let full_palette_mode = attr0.colors();
                        if full_palette_mode {
                            for byte_n in 0..8 {
                                let pixel_x_offset = x + byte_n;
                                // TODO: This access sometimes goes out of range if the mod is left out
                                let data = self.vram.borrow_mut().read(
                                    (0x4000 * 4
                                        + 32 * tile_n as usize
                                        + (if attr1.flip_v() { 7 - pixel_y } else { pixel_y }) * 8
                                        + (if attr1.flip_h() {
                                            7 - byte_n as usize
                                        } else {
                                            byte_n as usize
                                        }))
                                        % 0x18000,
                                );
                                let color = self
                                    .palette_ram
                                    .borrow_mut()
                                    .read_u16(0x200 + 2 * (data as usize));
                                if y as usize + pixel_y < 160 {
                                    let visible_with_windows = self.pixel_visible_with_windows(
                                        4, // OBJ layer
                                        pixel_x_offset as u16,
                                        y + pixel_y as u16,
                                    );
                                    if visible_with_windows
                                        && pixel_x_offset <= 239
                                        && color != background_color
                                    {
                                        self.framebuffer[(y as usize + pixel_y) * 480
                                            + 2 * (pixel_x_offset as usize)
                                            + 0] = color as u8;
                                        self.framebuffer[(y as usize + pixel_y) * 480
                                            + 2 * (pixel_x_offset as usize)
                                            + 1] = (color >> 8) as u8;
                                    }
                                }
                            }
                        } else {
                            for byte_n in 0..4 {
                                let pixel_x_offset = x + 2 * byte_n;
                                // TODO: This access sometimes goes out of range if the mod is left out
                                let data = self.vram.borrow_mut().read(
                                    (0x4000 * 4
                                        + 32 * tile_n as usize
                                        + (if attr1.flip_v() { 7 - pixel_y } else { pixel_y }) * 4
                                        + (if attr1.flip_h() {
                                            3 - byte_n as usize
                                        } else {
                                            byte_n as usize
                                        }))
                                        % 0x18000,
                                );
                                let mut color_i_left = data & 0b1111;
                                let mut color_left = self.palette_ram.borrow_mut().read_u16(
                                    0x200
                                        + 2 * (attr2.palette() as usize * 16
                                            + color_i_left as usize),
                                );
                                let mut color_i_right = (data >> 4) & 0b1111;
                                let mut color_right = self.palette_ram.borrow_mut().read_u16(
                                    0x200
                                        + 2 * (attr2.palette() as usize * 16
                                            + color_i_right as usize),
                                );
                                if attr1.flip_h() {
                                    std::mem::swap(&mut color_i_left, &mut color_i_right);
                                    std::mem::swap(&mut color_left, &mut color_right);
                                }
                                if y as usize + pixel_y < 160 {
                                    let visible_with_windows = self.pixel_visible_with_windows(
                                        4, // OBJ layer
                                        pixel_x_offset as u16,
                                        y + pixel_y as u16,
                                    );
                                    if visible_with_windows
                                        && pixel_x_offset <= 239
                                        && color_i_left != 0
                                    {
                                        self.framebuffer[(y as usize + pixel_y) * 480
                                            + 2 * (pixel_x_offset as usize)
                                            + 0] = color_left as u8;
                                        self.framebuffer[(y as usize + pixel_y) * 480
                                            + 2 * (pixel_x_offset as usize)
                                            + 1] = (color_left >> 8) as u8;
                                    }
                                    let visible_with_windows = self.pixel_visible_with_windows(
                                        4, // OBJ layer
                                        pixel_x_offset as u16 + 1,
                                        y + pixel_y as u16,
                                    );
                                    if visible_with_windows
                                        && (pixel_x_offset + 1) <= 239
                                        && color_i_right != 0
                                    {
                                        self.framebuffer[(y as usize + pixel_y) * 480
                                            + (2 * pixel_x_offset + 2) as usize] =
                                            color_right as u8;
                                        self.framebuffer[(y as usize + pixel_y) * 480
                                            + (2 * pixel_x_offset + 3) as usize] =
                                            (color_right >> 8) as u8;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

impl Memory for PPU {
    fn peek(&self, addr: usize) -> u8 {
        // TODO
        match addr {
            0x000 => self.lcd_control_reg.lo_byte(),
            0x001 => self.lcd_control_reg.hi_byte(),

            0x004 => self.lcd_status_reg.lo_byte(),
            0x005 => self.lcd_status_reg.hi_byte(),
            0x006 => self.scan_line,

            0x008 => self.bg_control_regs[0].lo_byte(),
            0x009 => self.bg_control_regs[0].hi_byte(),
            0x00A => self.bg_control_regs[1].lo_byte(),
            0x00B => self.bg_control_regs[1].hi_byte(),
            0x00C => self.bg_control_regs[2].lo_byte(),
            0x00D => self.bg_control_regs[2].hi_byte(),
            0x00E => self.bg_control_regs[3].lo_byte(),
            0x00F => self.bg_control_regs[3].hi_byte(),

            0x010 => self.scroll_regs[0].0.lo_byte(),
            0x011 => self.scroll_regs[0].0.hi_byte(),
            0x012 => self.scroll_regs[0].1.lo_byte(),
            0x013 => self.scroll_regs[0].1.hi_byte(),
            0x014 => self.scroll_regs[1].0.lo_byte(),
            0x015 => self.scroll_regs[1].0.hi_byte(),
            0x016 => self.scroll_regs[1].1.lo_byte(),
            0x017 => self.scroll_regs[1].1.hi_byte(),
            0x018 => self.scroll_regs[2].0.lo_byte(),
            0x019 => self.scroll_regs[2].0.hi_byte(),
            0x01A => self.scroll_regs[2].1.lo_byte(),
            0x01B => self.scroll_regs[2].1.hi_byte(),
            0x01C => self.scroll_regs[3].0.lo_byte(),
            0x01D => self.scroll_regs[3].0.hi_byte(),
            0x01E => self.scroll_regs[3].1.lo_byte(),
            0x01F => self.scroll_regs[3].1.hi_byte(),

            0x040 => self.win0_coords.0.lo_byte(),
            0x041 => self.win0_coords.0.hi_byte(),
            0x042 => self.win1_coords.0.lo_byte(),
            0x043 => self.win1_coords.0.hi_byte(),
            0x044 => self.win0_coords.1.lo_byte(),
            0x045 => self.win0_coords.1.hi_byte(),
            0x046 => self.win1_coords.1.lo_byte(),
            0x047 => self.win1_coords.1.hi_byte(),
            0x048 => self.win_inside.lo_byte(),
            0x049 => self.win_inside.hi_byte(),
            0x04A => self.win_outside.lo_byte(),
            0x04B => self.win_outside.hi_byte(),
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        // TODO
        match addr {
            0x000 => self.lcd_control_reg.set_lo_byte(data),
            0x001 => self.lcd_control_reg.set_hi_byte(data),

            0x004 => self.lcd_status_reg.set_lo_byte(data & 0b11111000),
            0x005 => self.lcd_status_reg.set_hi_byte(data),

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
            0x018 => self.scroll_regs[2].0.set_lo_byte(data),
            0x019 => self.scroll_regs[2].0.set_hi_byte(data),
            0x01A => self.scroll_regs[2].1.set_lo_byte(data),
            0x01B => self.scroll_regs[2].1.set_hi_byte(data),
            0x01C => self.scroll_regs[3].0.set_lo_byte(data),
            0x01D => self.scroll_regs[3].0.set_hi_byte(data),
            0x01E => self.scroll_regs[3].1.set_lo_byte(data),
            0x01F => self.scroll_regs[3].1.set_hi_byte(data),

            0x040 => self.win0_coords.0.set_lo_byte(data),
            0x041 => self.win0_coords.0.set_hi_byte(data),
            0x042 => self.win1_coords.0.set_lo_byte(data),
            0x043 => self.win1_coords.0.set_hi_byte(data),
            0x044 => self.win0_coords.1.set_lo_byte(data),
            0x045 => self.win0_coords.1.set_hi_byte(data),
            0x046 => self.win1_coords.1.set_lo_byte(data),
            0x047 => self.win1_coords.1.set_hi_byte(data),
            0x048 => self.win_inside.set_lo_byte(data),
            0x049 => self.win_inside.set_hi_byte(data),
            0x04A => self.win_outside.set_lo_byte(data),
            0x04B => self.win_outside.set_hi_byte(data),
            _ => {}
        }
    }
}
