bitfield! {
  /// 400004Ah - WININ
  /// Controls the effects and layers visible outside the windows,
  /// and inside the object window
  pub struct WinOutsideControlReg(u16);
  impl Debug;
  pub bg_enable, _: 3, 0;
  pub obj_enable, _: 4;
  pub color_effect_enable, _: 5;
  pub obj_bg_enable, _: 11, 8;
  pub obj_obj_enable, _: 12;
  pub obj_color_effect_enable, _: 13;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
