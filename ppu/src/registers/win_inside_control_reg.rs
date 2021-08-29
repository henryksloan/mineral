bitfield! {
  /// 4000048h - WININ
  /// Controls the effects and layers visible inside each window
  pub struct WinInsideControlReg(u16);
  impl Debug;
  pub win0_bg_enable, _: 3, 0;
  pub win0_obj_enable, _: 4;
  pub win0_color_effect_enable, _: 5;
  pub win1_bg_enable, _: 11, 8;
  pub win1_obj_enable, _: 12;
  pub win1_color_effect_enable, _: 13;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
