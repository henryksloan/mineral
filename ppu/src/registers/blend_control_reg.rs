bitfield! {
  /// 4000050h - BLDCNT
  /// Configures layer blending
  pub struct BlendControlReg(u16);
  impl Debug;
  pub source_bg0, _: 0;
  pub source_bg1, _: 1;
  pub source_bg2, _: 2;
  pub source_bg3, _: 3;
  pub source_obj, _: 4;
  pub source_bd, _: 5;
  pub mode, _: 7, 6;
  pub target_bg0, _: 8;
  pub target_bg1, _: 9;
  pub target_bg2, _: 10;
  pub target_bg3, _: 11;
  pub target_obj, _: 12;
  pub target_bd, _: 13;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
