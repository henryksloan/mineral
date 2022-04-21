bitfield! {
  /// 40000{2,3}{8,C}h - BG{2,3}{X,Y}
  /// Sets the reference point of an affine background
  #[derive(Clone, Copy)]
  pub struct BgRefReg(u32);
  impl Debug;
  pub fraction, _: 7, 0;
  pub integer, _ : 26, 8;
  pub sign, _: 27;
  pub value, _: 27, 0;

  pub u8, byte_0, set_byte_0: 7, 0;
  pub u8, byte_1, set_byte_1: 15, 8;
  pub u8, byte_2, set_byte_2: 23, 16;
  pub u8, byte_3, set_byte_3: 31, 24;
}

impl BgRefReg {
    pub fn signed_value(&self) -> i32 {
        ((self.value() << 4) as i32) >> 4
    }
}
