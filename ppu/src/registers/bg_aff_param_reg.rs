bitfield! {
  /// 40000{2,3}{8,C}h - BG{2,3}{X,Y}
  /// Sets the reference point of an affine background
  pub struct BgAffParamReg(u16);
  impl Debug;
  pub fraction, _: 7, 0;
  pub integer, _ : 14, 8;
  pub sign, _: 15;
  pub value, _: 15, 0;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

impl BgAffParamReg {
    pub fn signed_value(&self) -> i32 {
        self.value() as i16 as i32
    }
}
