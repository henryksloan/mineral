#[macro_use]
extern crate bitfield;

mod obj;
mod registers;
mod win;

use registers::*;

use std::cell::RefCell;
use std::rc::Rc;
use std::{cmp, iter};

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
    bg_ref_regs: [(BgRefReg, BgRefReg); 2],
    bg_aff_param_regs: [(BgAffParamReg, BgAffParamReg, BgAffParamReg, BgAffParamReg); 2],

    mosaic_reg: MosaicReg,
    blend_control_reg: BlendControlReg,
    blend_alpha_reg: BlendAlphaReg,
    blend_fade_reg: BlendFadeReg,

    win0_coords: (WinCoordReg, WinCoordReg),
    win1_coords: (WinCoordReg, WinCoordReg),
    win_inside: WinInsideControlReg,
    win_outside: WinOutsideControlReg,

    bg_ref_internal: [(i32, i32); 2],
    // Stores whether the pixels of the current scanline are visible with windows
    // The array elements are: BG 0-3, OBJ, Blend
    win_mask_bufs: [[bool; 6]; 240],

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
            bg_ref_regs: [(BgRefReg(0), BgRefReg(0)), (BgRefReg(0), BgRefReg(0))],
            bg_aff_param_regs: [
                (
                    BgAffParamReg(0),
                    BgAffParamReg(0),
                    BgAffParamReg(0),
                    BgAffParamReg(0),
                ),
                (
                    BgAffParamReg(0),
                    BgAffParamReg(0),
                    BgAffParamReg(0),
                    BgAffParamReg(0),
                ),
            ],

            mosaic_reg: MosaicReg(0),
            blend_control_reg: BlendControlReg(0),
            blend_alpha_reg: BlendAlphaReg(0),
            blend_fade_reg: BlendFadeReg(0),

            win0_coords: (WinCoordReg(0), WinCoordReg(0)),
            win1_coords: (WinCoordReg(0), WinCoordReg(0)),
            win_inside: WinInsideControlReg(0),
            win_outside: WinOutsideControlReg(0),

            bg_ref_internal: [(0, 0); 2],
            win_mask_bufs: [[false; 6]; 240],

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

        if self.scan_cycle == 1231 {
            if self.scan_line < 160 {
                for i in 0..2 {
                    self.bg_ref_internal[i].0 += self.bg_aff_param_regs[i].1.signed_value();
                    self.bg_ref_internal[i].1 += self.bg_aff_param_regs[i].3.signed_value();
                }
            } else {
                for i in 0..2 {
                    self.bg_ref_internal[i] = (
                        self.bg_ref_regs[i].0.signed_value(),
                        self.bg_ref_regs[i].1.signed_value(),
                    );
                }
            }
        }

        self.lcd_status_reg
            .set_vblank((160..=226).contains(&self.scan_line));
        self.lcd_status_reg.set_hblank(self.scan_cycle >= 960);
        self.lcd_status_reg
            .set_vcounter(self.scan_line == self.lcd_status_reg.vcounter_line());

        (vblank, hblank, vblank_irq, hblank_irq, vcounter_irq)
    }

    fn draw_scanline(&mut self) {
        self.update_win_masks_buf();

        let bgs_enabled = [
            self.lcd_control_reg.enable_bg0(),
            self.lcd_control_reg.enable_bg1(),
            self.lcd_control_reg.enable_bg2(),
            self.lcd_control_reg.enable_bg3(),
        ];
        let bg_lines = match self.lcd_control_reg.bg_mode() {
            0 => [
                Some(self.get_text_bg_scanline(0)),
                Some(self.get_text_bg_scanline(1)),
                Some(self.get_text_bg_scanline(2)),
                Some(self.get_text_bg_scanline(3)),
            ],
            1 => [
                Some(self.get_text_bg_scanline(0)),
                Some(self.get_text_bg_scanline(1)),
                Some(self.get_affine_text_bg_scanline(2)),
                None,
            ],
            2 => [
                None,
                None,
                Some(self.get_affine_text_bg_scanline(2)),
                Some(self.get_affine_text_bg_scanline(3)),
            ],
            3 => [
                None,
                None,
                Some(self.get_bitmap_bg_scanline(false, false, true)),
                None,
            ],
            4 => [
                None,
                None,
                Some(self.get_bitmap_bg_scanline(false, true, false)),
                None,
            ],
            5 => [
                None,
                None,
                Some(self.get_bitmap_bg_scanline(true, true, true)),
                None,
            ],
            _ => panic!("illegal video mode: {}", self.lcd_control_reg.bg_mode()),
        };
        let lines = (0..4)
            .filter(|&i| bgs_enabled[i])
            .map(|i| ((self.bg_control_regs[i].priority(), i), bg_lines[i]))
            .filter_map(|(prio_i, line_opt)| line_opt.map(|line| (prio_i, line)))
            .collect::<Vec<((u16, usize), [Option<(u8, u8)>; 240])>>();
        let sprite_line = self.get_sprite_scanline();

        let first_pixel_i = 480 * self.scan_line as usize;
        let backdrop_color = {
            let backdrop_color = self.palette_ram.borrow_mut().read_u16(0);
            ((backdrop_color & 0xFF) as u8, (backdrop_color >> 8) as u8)
        };
        let backdrop_pixel = {
            (
                (4, 5), // Backdrop has lower priority than any layer, and is "layer 5" in blending
                backdrop_color,
            )
        };

        let blend_mode = self.blend_control_reg.mode();
        let blend_source_mask = self.blend_control_reg.lo_byte() & 0b111111;
        let blend_target_mask = self.blend_control_reg.hi_byte() & 0b111111;
        let eva = self.blend_alpha_reg.eva().min(16);
        let evb = self.blend_alpha_reg.evb().min(16);
        let ey = self.blend_fade_reg.ey().min(16);

        for i in 0..240 {
            let bg_pixels = lines
                .iter()
                .filter_map(|line| line.1[i].map(|color| (line.0, color)));
            let mut pixels: Vec<((u16, usize), (u8, u8))> =
                if let Some((sprite_prio, sprite_color)) = sprite_line[i] {
                    iter::once(((sprite_prio, 4), sprite_color)) // OBJ are "layer 4" in blending
                        .chain(bg_pixels)
                        .collect()
                } else {
                    bg_pixels.collect()
                };
            pixels.sort_by_key(|((prio, layer), _)| {
                (*prio, if *layer == 4 { 0 } else { *layer }) // Sprites of priority X are on top of of layer X
            });
            pixels.push(backdrop_pixel);

            let inside_blend_window = self.win_mask_bufs[i][5];
            let windowed_blend_mode = if inside_blend_window {
                blend_mode
            } else {
                0b00
            };

            let color = match windowed_blend_mode {
                0b00 => pixels[0].1, // No blending
                0b01 => {
                    // Alpha blending
                    let mut candidates = pixels.into_iter().map(|((_, layer_n), color)| {
                        let is_source = (blend_source_mask >> layer_n) & 1 == 1;
                        let is_target = (blend_target_mask >> layer_n) & 1 == 1;
                        ((is_source, is_target), color)
                    });
                    let top = candidates.next();
                    if let Some(top) = top {
                        if !(top.0).0 {
                            top.1 // If the topmost pixel is not a source pixel, blending does not occur
                        } else {
                            let bottom = candidates
                                .next()
                                .and_then(|bottom| (bottom.0).1.then(|| bottom));
                            if let Some(bottom) = bottom {
                                // Blend
                                // TODO: Refactor to a function so it can be reused for black and white blending
                                let top_color = (((top.1).1 as u16) << 8) | ((top.1).0 as u16);
                                let (ar, ag, ab) = (
                                    (top_color >> 10) & 0x1F,
                                    (top_color >> 5) & 0x1F,
                                    top_color & 0x1F,
                                );
                                let bot_color =
                                    (((bottom.1).1 as u16) << 8) | ((bottom.1).0 as u16);
                                let (br, bg, bb) = (
                                    (bot_color >> 10) & 0x1F,
                                    (bot_color >> 5) & 0x1F,
                                    bot_color & 0x1F,
                                );
                                let (cr, cg, cb) = (
                                    cmp::min((ar * eva + br * evb) >> 4, 31),
                                    cmp::min((ag * eva + bg * evb) >> 4, 31),
                                    cmp::min((ab * eva + bb * evb) >> 4, 31),
                                );
                                let blended = (cr << 10) | (cg << 5) | cb;
                                ((blended & 0xFF) as u8, (blended >> 8) as u8)
                            } else {
                                top.1
                            }
                        }
                    } else {
                        backdrop_color
                    }
                }
                0b10 => {
                    // Fade to white
                    let mut candidates = pixels.into_iter().map(|((_, layer_n), color)| {
                        let is_source = (blend_source_mask >> layer_n) & 1 == 1;
                        (is_source, color)
                    });
                    let top = candidates.next();
                    if let Some(top) = top {
                        if !top.0 {
                            top.1 // If the topmost pixel is not a source pixel, blending does not occur
                        } else {
                            let top_color = (((top.1).1 as u16) << 8) | ((top.1).0 as u16);
                            let (ar, ag, ab) = (
                                (top_color >> 10) & 0x1F,
                                (top_color >> 5) & 0x1F,
                                top_color & 0x1F,
                            );
                            let fade_value = 0x1F * ey;
                            let (cr, cg, cb) = (
                                cmp::min((ar * (16 - ey) + fade_value) >> 4, 31),
                                cmp::min((ag * (16 - ey) + fade_value) >> 4, 31),
                                cmp::min((ab * (16 - ey) + fade_value) >> 4, 31),
                            );
                            let blended = (cr << 10) | (cg << 5) | cb;
                            ((blended & 0xFF) as u8, (blended >> 8) as u8)
                        }
                    } else {
                        backdrop_color
                    }
                }
                0b11 | _ => {
                    // Fade to black
                    let mut candidates = pixels.into_iter().map(|((_, layer_n), color)| {
                        let is_source = (blend_source_mask >> layer_n) & 1 == 1;
                        (is_source, color)
                    });
                    let top = candidates.next();
                    if let Some(top) = top {
                        if !top.0 {
                            top.1 // If the topmost pixel is not a source pixel, blending does not occur
                        } else {
                            let top_color = (((top.1).1 as u16) << 8) | ((top.1).0 as u16);
                            let (ar, ag, ab) = (
                                (top_color >> 10) & 0x1F,
                                (top_color >> 5) & 0x1F,
                                top_color & 0x1F,
                            );
                            let (cr, cg, cb) = (
                                cmp::min((ar * (16 - ey)) >> 4, 31),
                                cmp::min((ag * (16 - ey)) >> 4, 31),
                                cmp::min((ab * (16 - ey)) >> 4, 31),
                            );
                            let blended = (cr << 10) | (cg << 5) | cb;
                            ((blended & 0xFF) as u8, (blended >> 8) as u8)
                        }
                    } else {
                        backdrop_color
                    }
                }
            };

            let pixel_i = first_pixel_i + 2 * i;
            self.framebuffer[pixel_i + 0] = color.0;
            self.framebuffer[pixel_i + 1] = color.1;
        }
    }

    fn get_bitmap_bg_scanline(
        &self,
        small: bool,
        double_buffered: bool,
        full_palette_mode: bool,
    ) -> [Option<(u8, u8)>; 240] {
        // TODO: Mosaic
        let mut out = [None; 240];

        let page_base = if double_buffered && self.lcd_control_reg.frame_select() {
            0xA000
        } else {
            0
        };

        let (pa, pc) = (
            self.bg_aff_param_regs[0].0.signed_value(),
            self.bg_aff_param_regs[0].2.signed_value(),
        );

        let width = if small { 160 } else { 240 };
        let height = if small { 128 } else { 160 };
        for x in 0..240 {
            let visible_with_windows = self.win_mask_bufs[x][2];
            if !visible_with_windows {
                continue;
            }

            let px = (self.bg_ref_internal[0].0 + pa * x as i32) >> 8;
            let py = (self.bg_ref_internal[0].1 + pc * x as i32) >> 8;

            if !self.bg_control_regs[2].display_overflow() && (px < 0 || py < 0) {
                continue;
            }

            let (tex_x, tex_y) = {
                let mut tex_x = px;
                let mut tex_y = py;
                if self.bg_control_regs[2].display_overflow() {
                    tex_x = tex_x.rem_euclid(width as i32);
                    tex_y = tex_y.rem_euclid(height as i32);
                }
                (tex_x as usize, tex_y as usize)
            };

            if tex_x >= width || tex_y >= height {
                continue;
            }

            let color = if full_palette_mode {
                self.vram
                    .borrow_mut()
                    .read_u16(page_base + 2 * (tex_x + width * tex_y))
            } else {
                let color_i = self
                    .vram
                    .borrow_mut()
                    .read(page_base + tex_x + width * tex_y);
                self.palette_ram
                    .borrow_mut()
                    .read_u16(2 * (color_i as usize))
            };

            out[x] = Some((color as u8, (color >> 8) as u8));
        }
        out
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

        let (stretch_x, stretch_y) = if self.bg_control_regs[bg_n].mosaic() {
            let reg = &self.mosaic_reg;
            (reg.bg_h() as usize + 1, reg.bg_v() as usize + 1)
        } else {
            (1, 1)
        };
        let adjusted_y = {
            let offset_line = self.scan_line as usize + offset_y;
            offset_line - (offset_line % stretch_y)
        };

        let map_base = self.bg_control_regs[bg_n].screen_block() as usize * 0x800;
        let tile_row = (adjusted_y / 8) % n_bg_rows;
        let row = adjusted_y % 8;

        for ix in 0..240 {
            let visible_with_windows = self.win_mask_bufs[ix][bg_n];
            if !visible_with_windows {
                continue;
            }

            let tex_x = {
                let tex_x = offset_x + ix;
                tex_x - (tex_x % stretch_x)
            };
            let tile_col = tex_x / 8;
            let pixel_n = tex_x % 8;

            let screen_offset_x = if n_bg_cols == 64 && 31 < tile_col && tile_col < 64 {
                0x800
            } else {
                0
            };
            let screen_offset_y = if n_bg_rows == 64 && 31 < tile_row && tile_row < 64 {
                (n_bg_cols / 32) * 0x800
            } else {
                0
            };
            let map_entry = self.vram.borrow_mut().read_u16(
                map_base
                    + screen_offset_x
                    + screen_offset_y
                    + 2 * ((tile_row % 32) * 32 + (tile_col % 32)),
            );
            let tile_n = map_entry & 0b11_1111_1111;
            let flip_h = (map_entry >> 10) & 1 == 1;
            let flip_v = (map_entry >> 11) & 1 == 1;
            if full_palette_mode {
                let data = self.vram.borrow_mut().read(
                    0x4000 * self.bg_control_regs[bg_n].char_block() as usize
                        + 64 * tile_n as usize
                        + (if flip_v { 7 - row } else { row }) * 8
                        + (if flip_h { 7 - pixel_n } else { pixel_n }),
                );
                if data != 0 {
                    let color = self.palette_ram.borrow_mut().read_u16(2 * data as usize);
                    out[ix] = Some((color as u8, (color >> 8) as u8));
                }
            } else {
                let palette_n = (map_entry >> 12) & 0b1111;
                let byte_n = pixel_n / 2;
                let is_left = (pixel_n % 2) == 0;
                let data = self.vram.borrow_mut().read(
                    0x4000 * self.bg_control_regs[bg_n].char_block() as usize
                        + 32 * tile_n as usize
                        + (if flip_v { 7 - row } else { row }) * 4
                        + (if flip_h { 3 - byte_n } else { byte_n }),
                );
                let color_i = if is_left ^ flip_h {
                    data & 0b1111
                } else {
                    (data >> 4) & 0b1111
                };
                let color = self
                    .palette_ram
                    .borrow_mut()
                    .read_u16(2 * (palette_n as usize * 16 + color_i as usize));

                if color_i != 0 {
                    out[ix] = Some((color as u8, (color >> 8) as u8));
                }
            }
        }

        out
    }

    fn get_affine_text_bg_scanline(&self, bg_n: usize) -> [Option<(u8, u8)>; 240] {
        // TODO: Mosaic
        let mut out = [None; 240];

        let ctrl = &self.bg_control_regs[bg_n];

        let (pa, pc) = (
            self.bg_aff_param_regs[bg_n - 2].0.signed_value(),
            self.bg_aff_param_regs[bg_n - 2].2.signed_value(),
        );

        let map_base = ctrl.screen_block() as usize * 0x800;

        let (n_bg_cols, n_bg_rows) = match ctrl.size() {
            0b00 => (16, 16),
            0b01 => (32, 32),
            0b10 => (64, 64),
            0b11 | _ => (128, 128),
        };

        for ix in 0..240 {
            let visible_with_windows = self.win_mask_bufs[ix as usize][bg_n];
            if !visible_with_windows {
                continue;
            }

            let px = (self.bg_ref_internal[bg_n - 2].0 + pa * ix) >> 8;
            let py = (self.bg_ref_internal[bg_n - 2].1 + pc * ix) >> 8;

            if !ctrl.display_overflow() && (px < 0 || py < 0) {
                continue;
            }

            let (tile_col, tile_row) = {
                let mut tile_col = px / 8;
                let mut tile_row = py / 8;
                if ctrl.display_overflow() {
                    tile_col = tile_col.rem_euclid(n_bg_cols as i32);
                    tile_row = tile_row.rem_euclid(n_bg_rows as i32);
                }
                (tile_col as usize, tile_row as usize)
            };

            // TODO: Can we break in one of these cases?
            if tile_col >= n_bg_cols || tile_row >= n_bg_rows {
                continue;
            }

            let tile_n = self
                .vram
                .borrow_mut()
                .read(map_base + tile_row * n_bg_cols + tile_col);
            let data = self.vram.borrow_mut().read(
                0x4000 * ctrl.char_block() as usize
                    + 64 * tile_n as usize
                    + (py as usize % 8) * 8
                    + (px as usize % 8),
            );
            if data != 0 {
                let color = self.palette_ram.borrow_mut().read_u16(2 * data as usize);
                out[ix as usize] = Some((color as u8, (color >> 8) as u8));
            }
        }

        out
    }
}

impl Memory for PPU {
    fn peek(&self, addr: usize) -> u8 {
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

            0x020 => self.bg_aff_param_regs[0].0.lo_byte(),
            0x021 => self.bg_aff_param_regs[0].0.hi_byte(),
            0x022 => self.bg_aff_param_regs[0].1.lo_byte(),
            0x023 => self.bg_aff_param_regs[0].1.hi_byte(),
            0x024 => self.bg_aff_param_regs[0].2.lo_byte(),
            0x025 => self.bg_aff_param_regs[0].2.hi_byte(),
            0x026 => self.bg_aff_param_regs[0].3.lo_byte(),
            0x027 => self.bg_aff_param_regs[0].3.hi_byte(),
            0x028 => self.bg_ref_regs[0].0.byte_0(),
            0x029 => self.bg_ref_regs[0].0.byte_1(),
            0x02A => self.bg_ref_regs[0].0.byte_2(),
            0x02B => self.bg_ref_regs[0].0.byte_3(),
            0x02C => self.bg_ref_regs[0].1.byte_0(),
            0x02D => self.bg_ref_regs[0].1.byte_1(),
            0x02E => self.bg_ref_regs[0].1.byte_2(),
            0x02F => self.bg_ref_regs[0].1.byte_3(),
            0x030 => self.bg_aff_param_regs[1].0.lo_byte(),
            0x031 => self.bg_aff_param_regs[1].0.hi_byte(),
            0x032 => self.bg_aff_param_regs[1].1.lo_byte(),
            0x033 => self.bg_aff_param_regs[1].1.hi_byte(),
            0x034 => self.bg_aff_param_regs[1].2.lo_byte(),
            0x035 => self.bg_aff_param_regs[1].2.hi_byte(),
            0x036 => self.bg_aff_param_regs[1].3.lo_byte(),
            0x037 => self.bg_aff_param_regs[1].3.hi_byte(),
            0x038 => self.bg_ref_regs[1].0.byte_0(),
            0x039 => self.bg_ref_regs[1].0.byte_1(),
            0x03A => self.bg_ref_regs[1].0.byte_2(),
            0x03B => self.bg_ref_regs[1].0.byte_3(),
            0x03C => self.bg_ref_regs[1].1.byte_0(),
            0x03D => self.bg_ref_regs[1].1.byte_1(),
            0x03E => self.bg_ref_regs[1].1.byte_2(),
            0x03F => self.bg_ref_regs[1].1.byte_3(),

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

            0x50 => self.blend_control_reg.lo_byte(),
            0x51 => self.blend_control_reg.hi_byte(),
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
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

            0x020 => self.bg_aff_param_regs[0].0.set_lo_byte(data),
            0x021 => self.bg_aff_param_regs[0].0.set_hi_byte(data),
            0x022 => self.bg_aff_param_regs[0].1.set_lo_byte(data),
            0x023 => self.bg_aff_param_regs[0].1.set_hi_byte(data),
            0x024 => self.bg_aff_param_regs[0].2.set_lo_byte(data),
            0x025 => self.bg_aff_param_regs[0].2.set_hi_byte(data),
            0x026 => self.bg_aff_param_regs[0].3.set_lo_byte(data),
            0x027 => self.bg_aff_param_regs[0].3.set_hi_byte(data),
            0x028 => self.bg_ref_regs[0].0.set_byte_0(data),
            0x029 => self.bg_ref_regs[0].0.set_byte_1(data),
            0x02A => self.bg_ref_regs[0].0.set_byte_2(data),
            0x02B => self.bg_ref_regs[0].0.set_byte_3(data),
            0x02C => self.bg_ref_regs[0].1.set_byte_0(data),
            0x02D => self.bg_ref_regs[0].1.set_byte_1(data),
            0x02E => self.bg_ref_regs[0].1.set_byte_2(data),
            0x02F => self.bg_ref_regs[0].1.set_byte_3(data),
            0x030 => self.bg_aff_param_regs[1].0.set_lo_byte(data),
            0x031 => self.bg_aff_param_regs[1].0.set_hi_byte(data),
            0x032 => self.bg_aff_param_regs[1].1.set_lo_byte(data),
            0x033 => self.bg_aff_param_regs[1].1.set_hi_byte(data),
            0x034 => self.bg_aff_param_regs[1].2.set_lo_byte(data),
            0x035 => self.bg_aff_param_regs[1].2.set_hi_byte(data),
            0x036 => self.bg_aff_param_regs[1].3.set_lo_byte(data),
            0x037 => self.bg_aff_param_regs[1].3.set_hi_byte(data),
            0x038 => self.bg_ref_regs[1].0.set_byte_0(data),
            0x039 => self.bg_ref_regs[1].0.set_byte_1(data),
            0x03A => self.bg_ref_regs[1].0.set_byte_2(data),
            0x03B => self.bg_ref_regs[1].0.set_byte_3(data),
            0x03C => self.bg_ref_regs[1].1.set_byte_0(data),
            0x03D => self.bg_ref_regs[1].1.set_byte_1(data),
            0x03E => self.bg_ref_regs[1].1.set_byte_2(data),
            0x03F => self.bg_ref_regs[1].1.set_byte_3(data),

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

            0x4C => self.mosaic_reg.set_lo_byte(data),
            0x4D => self.mosaic_reg.set_hi_byte(data),
            0x50 => self.blend_control_reg.set_lo_byte(data),
            0x51 => self.blend_control_reg.set_hi_byte(data),
            0x52 => self.blend_alpha_reg.set_lo_byte(data),
            0x53 => self.blend_alpha_reg.set_hi_byte(data),
            0x54 => self.blend_fade_reg.set_lo_byte(data),
            0x55 => self.blend_fade_reg.set_hi_byte(data),
            _ => {}
        }

        if addr >= 0x26 && addr <= 0x3F {
            let i = (addr - 0x26) / 0x10;
            self.bg_ref_internal[i] = (
                self.bg_ref_regs[i].0.signed_value(),
                self.bg_ref_regs[i].1.signed_value(),
            );
        }
    }
}
