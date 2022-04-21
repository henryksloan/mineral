use crate::PPU;

impl PPU {
    pub(super) fn get_bitmap_bg_scanline(
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

    pub(super) fn get_text_bg_scanline(&self, bg_n: usize) -> [Option<(u8, u8)>; 240] {
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

    pub(super) fn get_affine_text_bg_scanline(&self, bg_n: usize) -> [Option<(u8, u8)>; 240] {
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
