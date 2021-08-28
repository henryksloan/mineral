bitfield! {
  /// 4000000h - DISPCNT
  /// Sets global rules and flags for rendering
  pub struct LcdControlReg(u16);
  impl Debug;
  pub bg_mode, _: 2, 0; // TODO: convert into enum
  cgb_mode, _ : 3;
  pub frame_select, _: 4;
  pub hblank_free, _: 5;
  pub obj_char_mapping, _: 6;
  pub forced_blank, _: 7;
  pub enable_bg0, _: 8;
  pub enable_bg1, _: 9;
  pub enable_bg2, _: 10;
  pub enable_bg3, _: 11;
  pub enable_obj, _: 12;
  pub enable_win0, _: 13;
  pub enable_win1, _: 14;
  pub enable_winobj, _: 15;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
