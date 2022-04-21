pub mod obj_attrs;

use obj_attrs::*;

use crate::PPU;

impl PPU {
    pub(super) fn get_sprite_scanline(&self) -> [Option<(u16, (u8, u8))>; 240] {
        let mut out = [None; 240];

        if !self.lcd_control_reg.enable_obj() {
            return out;
        }

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
            let attrs = self.get_obj_attrs(sprite_n);

            if matches!(attrs.0.mode(), ObjMode::Window) {
                continue;
            }

            if attrs.0.affine() {
                self.render_affine_sprite_scanline(&mut out, attrs);
            } else {
                self.render_regular_sprite_scanline(&mut out, attrs);
            }
        }

        out
    }

    pub(super) fn get_obj_win_scanline(&self) -> [bool; 240] {
        if !self.lcd_control_reg.enable_obj() || !self.lcd_control_reg.enable_winobj() {
            return [false; 240];
        }

        let mut out = [None; 240];
        for sprite_n in 0..128 {
            let attrs = self.get_obj_attrs(sprite_n);

            if !matches!(attrs.0.mode(), ObjMode::Window) {
                continue;
            }

            if attrs.0.affine() {
                self.render_affine_sprite_scanline(&mut out, attrs);
            } else {
                self.render_regular_sprite_scanline(&mut out, attrs);
            }
        }

        out.map(|pixel| pixel.is_some())
    }

    fn render_regular_sprite_scanline(
        &self,
        line_buf: &mut [Option<(u16, (u8, u8))>; 240],
        attrs: ObjAttrs,
    ) {
        if attrs.0.disable() {
            return;
        }

        let (n_cols, n_rows) = attrs.size();
        let (screen_x, screen_y) = attrs.screen_coords();

        let row = {
            let row = self.scan_line as i32 - screen_y;
            if row < 0 || row >= (n_rows * 8) as i32 {
                return;
            }
            row as usize
        };

        for col in 0..(n_cols as usize * 8) {
            let x = {
                let x = screen_x + col as i32;
                if x < 0 {
                    continue;
                }
                if x >= 240 {
                    break;
                }
                x as usize
            };

            let visible_with_windows = self.win_mask_bufs[x][4];
            if visible_with_windows {
                if let Some(pixel) = self.get_sprite_pixel(&attrs, row, col) {
                    line_buf[x] = Some(pixel);
                }
            }
        }
    }

    fn render_affine_sprite_scanline(
        &self,
        line_buf: &mut [Option<(u16, (u8, u8))>; 240],
        attrs: ObjAttrs,
    ) {
        let (pa, pb, pc, pd) = {
            let params_base = 32 * attrs.1.affine_params() as usize + 6;
            let mut oam = self.oam.borrow_mut();
            (
                oam.read_u16(params_base + 8 * 0) as i16 as i32,
                oam.read_u16(params_base + 8 * 1) as i16 as i32,
                oam.read_u16(params_base + 8 * 2) as i16 as i32,
                oam.read_u16(params_base + 8 * 3) as i16 as i32,
            )
        };

        let (ref_x, ref_y) = attrs.screen_coords();

        let size = attrs.size();
        let hwidth = size.0 as i32 * 4 * if attrs.0.double_size() { 2 } else { 1 };
        let hheight = size.1 as i32 * 4 * if attrs.0.double_size() { 2 } else { 1 };

        let y = self.scan_line as i32;
        let iy = y - (ref_y + hheight);

        if !(y >= ref_y && y < ref_y + hheight * 2) {
            return;
        }

        for ix in (-hwidth)..hwidth {
            let screen_x = ref_x + hwidth + ix;
            if screen_x < 0 {
                continue;
            }
            if screen_x >= 240 {
                break;
            }

            let px = (pa * ix + pb * iy) >> 8;
            let py = (pc * ix + pd * iy) >> 8;

            let tex_x = px + size.0 as i32 * 4;
            let tex_y = py + size.1 as i32 * 4;

            if (tex_x >= 0 && tex_x < size.0 as i32 * 8)
                && (tex_y >= 0 && tex_y < size.1 as i32 * 8)
            {
                let visible_with_windows = self.win_mask_bufs[screen_x as usize][4];
                if visible_with_windows {
                    if let Some(pixel) =
                        self.get_sprite_pixel(&attrs, tex_y as usize, tex_x as usize)
                    {
                        line_buf[screen_x as usize] = Some(pixel);
                    }
                }
            }
        }
    }

    fn get_sprite_pixel(
        &self,
        attrs: &ObjAttrs,
        row: usize,
        col: usize,
    ) -> Option<(u16, (u8, u8))> {
        let full_palette_mode = attrs.0.colors();
        let bytes_per_tile = if full_palette_mode { 2 } else { 1 };

        let (n_cols, n_rows) = attrs.size();
        let (flip_v, flip_h) = if attrs.0.affine() {
            (false, false)
        } else {
            (attrs.1.flip_v(), attrs.1.flip_h())
        };

        let (stretch_x, stretch_y) = if attrs.0.mosaic() {
            let reg = &self.mosaic_reg;
            (reg.obj_h() as usize + 1, reg.obj_v() as usize + 1)
        } else {
            (1, 1)
        };

        let (row, col) = (row - (row % stretch_y), col - (col % stretch_x));

        let tile_row = if flip_v {
            (n_rows as usize - 1) - (row / 8)
        } else {
            row / 8
        };
        let tile_row_start = {
            let distance_between_rows = if self.lcd_control_reg.obj_char_mapping() {
                n_cols as usize * bytes_per_tile // 1D: Rows immediately follow each other
            } else {
                0x20 // 2D: Rows are laid out vertically in the 32x32-tile charblock
            };
            attrs.2.tile() as usize + tile_row * distance_between_rows
        };

        let tile_col = if flip_h {
            (n_cols as usize - 1) - (col / 8)
        } else {
            col / 8
        };
        let tile_n = tile_row_start + tile_col * bytes_per_tile;

        let tile_start = 0x4000 * 4 + 32 * tile_n as usize; // Sprites start in charblock 4
        let pixel_row = if flip_v { 7 - (row % 8) } else { row % 8 };
        let pixel_col = if flip_h { 7 - (col % 8) } else { col % 8 };

        let color_i = if full_palette_mode {
            self.vram
                .borrow_mut()
                .read((tile_start + (8 * pixel_row) + pixel_col) % 0x18000)
        } else {
            let color_i_pair = self
                .vram
                .borrow_mut()
                .read((tile_start + (4 * pixel_row) + (pixel_col / 2)) % 0x18000);
            if pixel_col % 2 == 0 {
                color_i_pair & 0b1111
            } else {
                (color_i_pair >> 4) & 0b1111
            }
        };

        if color_i != 0 {
            let palette_start = if full_palette_mode {
                0
            } else {
                attrs.2.palette() as usize * 16
            };
            let color = self
                .palette_ram
                .borrow_mut()
                .read_u16(0x200 + 2 * (palette_start + color_i as usize));
            Some((attrs.2.priority(), (color as u8, (color >> 8) as u8)))
        } else {
            None
        }
    }

    fn get_obj_attrs(&self, sprite_n: usize) -> ObjAttrs {
        let oam_addr = 8 * sprite_n;
        let attr0 = ObjAttr0(self.oam.borrow_mut().read_u16(oam_addr + 0));
        let attr1 = ObjAttr1(self.oam.borrow_mut().read_u16(oam_addr + 2));
        let attr2 = ObjAttr2(self.oam.borrow_mut().read_u16(oam_addr + 4));
        ObjAttrs(attr0, attr1, attr2)
    }
}
