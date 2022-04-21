pub struct ObjAttrs(pub ObjAttr0, pub ObjAttr1, pub ObjAttr2);

impl ObjAttrs {
    pub fn size(&self) -> (u16, u16) {
        let size_map = match self.0.shape() {
            0 => [(1, 1), (2, 2), (4, 4), (8, 8)], // Square
            1 => [(2, 1), (4, 1), (4, 2), (8, 4)], // Horizontal
            2 => [(1, 2), (1, 4), (2, 4), (4, 8)], // Vertical
            _ => return (8, 8),                    // panic!("Illegal obj shape"),
        };
        size_map[self.1.size() as usize]
    }

    // The sprite's (x, y) position in screen-space;
    // negative values may be partially offscreen
    pub fn screen_coords(&self) -> (i32, i32) {
        let screen_x = {
            let mut x = self.1.x_coord() as i32;
            if x >= 240 {
                x -= 512;
            }
            x
        };
        let screen_y = {
            let mut y = self.0.y_coord() as i32;
            if y >= 160 {
                y -= 256;
            }
            y
        };

        (screen_x, screen_y)
    }
}

bitfield! {
  pub struct ObjAttr0(u16);
  impl Debug;
  pub y_coord, _: 7, 0;
  pub affine, _ : 8;
  // Double size in affine mode, disable in normal mode
  pub double_size, _: 9;
  pub disable, _: 9;
  pub mode, _: 11, 10;
  pub mosaic, _: 12;
  pub colors, _: 13;
  pub shape, _: 15, 14;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

bitfield! {
  pub struct ObjAttr1(u16);
  impl Debug;
  pub x_coord, _: 8, 0;
  // Affine params in affine mode, flip flags in normal mode
  pub affine_params, _ : 13, 9;
  pub flip_h, _ : 12;
  pub flip_v, _ : 13;
  // Double size in affine mode, disable in normal mode
  pub size, _: 15, 14;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

bitfield! {
  pub struct ObjAttr2(u16);
  impl Debug;
  pub tile, _: 9, 0;
  pub priority, _ : 11, 10;
  pub palette, _ : 15, 12;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
