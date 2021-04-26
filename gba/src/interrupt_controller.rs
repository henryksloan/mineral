use memory::Memory;

pub struct InterruptController {
    master_enable_reg: MasterEnableReg,
    enable_reg: InterruptEnableReg,
    pub request_reg: InterruptRequestReg,
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            master_enable_reg: MasterEnableReg(0),
            enable_reg: InterruptEnableReg(0),
            request_reg: InterruptRequestReg(0),
        }
    }

    pub fn has_interrupt(&self) -> bool {
        (self.request_reg.0 & 0b11_1111_1111_1111) != 0
    }
}

impl Memory for InterruptController {
    fn peek(&self, addr: usize) -> u8 {
        match addr {
            0x200 => self.enable_reg.lo_byte(),
            0x201 => self.enable_reg.hi_byte(),
            0x202 => self.request_reg.lo_byte(),
            0x203 => self.request_reg.hi_byte(),

            0x208 => self.master_enable_reg.byte_0(),
            0x209 => self.master_enable_reg.byte_1(),
            0x20A => self.master_enable_reg.byte_2(),
            0x20B => self.master_enable_reg.byte_3(),
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        match addr {
            0x200 => self.enable_reg.set_lo_byte(data),
            0x201 => self.enable_reg.set_hi_byte(data),
            0x202 => self.request_reg.set_lo_byte(data),
            0x203 => self.request_reg.set_hi_byte(data),

            0x208 => self.master_enable_reg.set_byte_0(data),
            0x209 => self.master_enable_reg.set_byte_1(data),
            0x20A => self.master_enable_reg.set_byte_2(data),
            0x20B => self.master_enable_reg.set_byte_3(data),
            _ => {}
        }
    }
}

bitfield! {
  /// 4000208h - Interrupt Master Enable Register
  /// If clear, all interrupt requests will be ignored
  pub struct MasterEnableReg(u32);
  impl Debug;
  pub enable, _: 0;

  pub u8, byte_0, set_byte_0: 7, 0;
  pub u8, byte_1, set_byte_1: 15, 8;
  pub u8, byte_2, set_byte_2: 23, 16;
  pub u8, byte_3, set_byte_3: 31, 24;
}

bitfield! {
  /// 4000200h - Interrupt Enable Register
  /// If a flag is clear, corresponding interrupts will be ignored
  pub struct InterruptEnableReg(u16);
  impl Debug;
  pub vblank, _: 0;
  pub hblank, _: 1;
  pub vcounter, _: 2;
  pub timer0, _: 3;
  pub timer1, _: 4;
  pub timer2, _: 5;
  pub timer3, _: 6;
  pub serial, _: 7;
  pub dma0, _: 8;
  pub dma1, _: 9;
  pub dma2, _: 10;
  pub dma3, _: 11;
  pub keypad, _: 12;
  pub gamepak, _: 13;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

bitfield! {
  /// 4000202h - Interrupt Enable Flags
  /// If clear, all interrupt requests will be ignored
  pub struct InterruptRequestReg(u16);
  impl Debug;
  pub vblank, set_vblank: 0;
  pub hblank, set_hblank: 1;
  pub vcounter, set_vcounter: 2;
  pub timer0, set_timer0: 3;
  pub timer1, set_timer1: 4;
  pub timer2, set_timer2: 5;
  pub timer3, set_timer3: 6;
  pub serial, set_serial: 7;
  pub dma0, set_dma0: 8;
  pub dma1, set_dma1: 9;
  pub dma2, set_dma2: 10;
  pub dma3, set_dma3: 11;
  pub keypad, set_keypad: 12;
  pub gamepak, set_gamepak: 13;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
