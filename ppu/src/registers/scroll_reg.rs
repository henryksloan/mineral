bitfield! {
  /// 400001Xh - BG{0,1,2,3}{H,V}OFS
  /// Sets a background's X and Y offset
  pub struct ScrollReg(u16);
  impl Debug;
  pub offset, _: 8, 0;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
