bitfield! {
  /// 4000008h, 400000Ah, 400000Ch, 400000Eh - BG{0,1,2,3}CNT
  /// Configures a particular background
  #[derive(Clone, Copy)]
  pub struct BgControlReg(u16);
  impl Debug;
  pub priority, _: 1, 0;
  pub char_block, _ : 3, 2;
  pub mosaic, _: 6;
  pub colors, _: 7;
  pub screen_block, _: 12, 8;
  pub display_overflow, _: 13;
  pub size, _: 15, 14;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
