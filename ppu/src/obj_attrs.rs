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
