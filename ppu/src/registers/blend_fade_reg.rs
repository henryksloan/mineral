bitfield! {
  /// 4000054h - BLDY
  /// Configures layer black/white blending
  pub struct BlendFadeReg(u16);
  impl Debug;
  pub ey, _: 4, 0;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
