bitfield! {
  /// 4000004h - DISPSTAT
  /// Display status and interrupt enables
  pub struct LcdStatusReg(u16);
  impl Debug;
  pub vblank, set_vblank: 0;
  pub hblank, set_hblank: 1;
  pub vcounter, set_vcounter: 2;
  pub vblank_irq, set_vblank_irq: 3;
  pub hblank_irq, set_hblank_irq: 4;
  pub vcounter_irq, set_vcounter_irq: 5;
  pub u8, vcounter_line, set_vcounter_line: 15, 8;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
