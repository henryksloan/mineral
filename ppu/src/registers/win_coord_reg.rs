bitfield! {
  /// 4000040h, 40000042h, 4000044h, 4000046h - WIN{0,1}{X,Y}
  /// Configures the X or Y coordinates of window 0 or 1
  pub struct WinCoordReg(u16);
  impl Debug;
  pub coord_hi, _: 7, 0;
  pub coord_lo, _: 15, 8;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
