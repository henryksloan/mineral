bitfield! {
  /// 4000052h - BLDALPHA
  /// Configures layer blending alpha coefficients
  pub struct BlendAlphaReg(u16);
  impl Debug;
  pub eva, _: 4, 0;
  pub evb, _: 12, 8;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
