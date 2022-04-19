bitfield! {
  /// 400004Ch - MOSAIC
  /// Configures background mosaic
  pub struct MosaicReg(u16);
  impl Debug;
  pub bg_h, _: 3, 0;
  pub bg_v, _: 7, 4;
  pub obj_h, _: 11, 8;
  pub obj_v, _: 15, 12;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
