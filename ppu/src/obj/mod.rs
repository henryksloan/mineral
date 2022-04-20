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
            let oam_addr = 8 * sprite_n;
            let attr0 = ObjAttr0(self.oam.borrow_mut().read_u16(oam_addr + 0));
            let attr1 = ObjAttr1(self.oam.borrow_mut().read_u16(oam_addr + 2));
            let attr2 = ObjAttr2(self.oam.borrow_mut().read_u16(oam_addr + 4));
            let attrs = ObjAttrs(attr0, attr1, attr2);

            if attrs.0.affine() {
                self.render_affine_sprite_scanline(&mut out, attrs);
            } else {
                self.render_regular_sprite_scanline(&mut out, attrs);
            }
        }

        out
    }

    fn render_regular_sprite_scanline(
        &self,
        line_buf: &mut [Option<(u16, (u8, u8))>; 240],
        attrs: ObjAttrs,
    ) {
        if attrs.0.disable() {
            return;
        }

        let bytes_per_tile = if attrs.0.colors() { 2 } else { 1 };
        let size = attrs.size();

        let y = self.scan_line as u16;

        let y_offset = if 256 - attrs.0.y_coord() < 8 * size.1 {
            if 8 * size.1 - (256 - attrs.0.y_coord()) <= y {
                return;
            } else {
                y + (256 - attrs.0.y_coord())
            }
        } else {
            if attrs.0.y_coord() > y || attrs.0.y_coord() + 8 * size.1 <= y {
                return;
            } else {
                y - attrs.0.y_coord()
            }
        };

        let pixel_y = (y_offset % 8) as usize;
        let row = y_offset / 8;
        let row_start = attrs.2.tile()
            + (if attrs.1.flip_v() {
                (size.1 - 1) - row
            } else {
                row
            }) * if self.lcd_control_reg.obj_char_mapping() {
                size.0 * bytes_per_tile // 1D
            } else {
                0x20 // 2D
            };

        for col in 0..size.0 {
            let tile_n = row_start
                + (if attrs.1.flip_h() {
                    (size.0 - 1) - col
                } else {
                    col
                }) * bytes_per_tile;
            let x = attrs.1.x_coord() + 8 * col;
            let full_palette_mode = attrs.0.colors();
            if full_palette_mode {
                for byte_n in 0..8 {
                    let pixel_x_offset = (x + byte_n) % 512;
                    // TODO: This access sometimes goes out of range if the mod is left out
                    let data = self.vram.borrow_mut().read(
                        (0x4000 * 4
                            + 32 * tile_n as usize
                            + (if attrs.1.flip_v() {
                                7 - pixel_y
                            } else {
                                pixel_y
                            }) * 8
                            + (if attrs.1.flip_h() {
                                7 - byte_n as usize
                            } else {
                                byte_n as usize
                            }))
                            % 0x18000,
                    );
                    if y as usize + pixel_y < 160 {
                        let visible_with_windows = self.pixel_visible_with_windows(
                            4, // OBJ layer
                            pixel_x_offset as u16,
                            y + pixel_y as u16,
                        );
                        if data != 0 && visible_with_windows && pixel_x_offset <= 239 {
                            let color = self
                                .palette_ram
                                .borrow_mut()
                                .read_u16(0x200 + 2 * (data as usize));
                            line_buf[pixel_x_offset as usize] =
                                Some((attrs.2.priority(), (color as u8, (color >> 8) as u8)));
                        }
                    }
                }
            } else {
                for byte_n in 0..4 {
                    let pixel_x_offset = (x + 2 * byte_n) % 512;
                    // TODO: This access sometimes goes out of range if the mod is left out
                    let data = self.vram.borrow_mut().read(
                        (0x4000 * 4
                            + 32 * tile_n as usize
                            + (if attrs.1.flip_v() {
                                7 - pixel_y
                            } else {
                                pixel_y
                            }) * 4
                            + (if attrs.1.flip_h() {
                                3 - byte_n as usize
                            } else {
                                byte_n as usize
                            }))
                            % 0x18000,
                    );
                    let mut color_i_left = data & 0b1111;
                    let mut color_left = self.palette_ram.borrow_mut().read_u16(
                        0x200 + 2 * (attrs.2.palette() as usize * 16 + color_i_left as usize),
                    );
                    let mut color_i_right = (data >> 4) & 0b1111;
                    let mut color_right = self.palette_ram.borrow_mut().read_u16(
                        0x200 + 2 * (attrs.2.palette() as usize * 16 + color_i_right as usize),
                    );
                    if attrs.1.flip_h() {
                        std::mem::swap(&mut color_i_left, &mut color_i_right);
                        std::mem::swap(&mut color_left, &mut color_right);
                    }
                    let visible_with_windows = self.pixel_visible_with_windows(
                        4, // OBJ layer
                        pixel_x_offset as u16,
                        y + pixel_y as u16, // TODO: Is it right to add pixel_y here and below?
                    );
                    if visible_with_windows && pixel_x_offset <= 239 && color_i_left != 0 {
                        line_buf[pixel_x_offset as usize] = Some((
                            attrs.2.priority(),
                            (color_left as u8, (color_left >> 8) as u8),
                        ));
                    }
                    let visible_with_windows = self.pixel_visible_with_windows(
                        4, // OBJ layer
                        pixel_x_offset as u16 + 1,
                        y + pixel_y as u16,
                    );
                    if visible_with_windows && (pixel_x_offset + 1) <= 239 && color_i_right != 0 {
                        line_buf[pixel_x_offset as usize + 1] = Some((
                            attrs.2.priority(),
                            (color_right as u8, (color_right >> 8) as u8),
                        ));
                    }
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

        let ref_x = {
            let mut x = attrs.1.x_coord() as i32;
            if x >= 240 {
                x -= 512;
            }
            x
        };
        let ref_y = {
            let mut y = attrs.0.y_coord() as i32;
            if y >= 160 {
                y -= 256;
            }
            y
        };

        let bytes_per_tile = if attrs.0.colors() { 2 } else { 1 };
        let size = attrs.size();
        let hwidth = size.0 as i32 * 4 * if attrs.0.double_size() { 2 } else { 1 };
        let hheight = size.1 as i32 * 4 * if attrs.0.double_size() { 2 } else { 1 };

        let y = self.scan_line as i32;
        let iy = y - (ref_y + hheight);

        if !(y >= ref_y && y < ref_y + hheight * 2) {
            return;
        }

        for ix in (-hwidth)..hwidth {
            // TODO: Most of the below can be refactored to some get_pixel function
            // and something like draw_pixel
            // same with regular sprites
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

            // TODO: Add 256-color sprites etc.
            if (tex_x >= 0 && tex_x < size.0 as i32 * 8)
                && (tex_y >= 0 && tex_y < size.1 as i32 * 8)
            {
                let row = tex_y as u16 / 8;
                let row_start = attrs.2.tile()
                    + row
                        * if self.lcd_control_reg.obj_char_mapping() {
                            size.0 * bytes_per_tile // 1D
                        } else {
                            0x20 // 2D
                        };
                let col = tex_x as u16 / 8;
                let tile_n = row_start + col * bytes_per_tile;

                let pixel_y = tex_y as usize % 8;
                let byte_n = ((tex_x as usize) / 2) % 4;
                let data = self.vram.borrow_mut().read(
                    (0x4000 * 4 + 32 * tile_n as usize + pixel_y * 4 + (byte_n as usize)) % 0x18000,
                );

                let color_i = match tex_x % 2 {
                    0 => data & 0b1111,            // Left pixel
                    1 | _ => (data >> 4) & 0b1111, // Right pixel
                } as usize;

                if color_i == 0 {
                    continue;
                }

                // TODO: Transparency?
                let color = self
                    .palette_ram
                    .borrow_mut()
                    .read_u16(0x200 + 2 * (attrs.2.palette() as usize * 16 + color_i));

                let visible_with_windows = self.pixel_visible_with_windows(
                    4, // OBJ layer
                    screen_x as u16 + 1,
                    y as u16,
                );
                if visible_with_windows {
                    line_buf[screen_x as usize] =
                        Some((attrs.2.priority(), (color as u8, (color >> 8) as u8)));
                }
            }
        }
    }
}
