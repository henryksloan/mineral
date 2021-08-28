use memory::Memory;

pub const IRQ_VBLANK: usize = 0x0;
pub const IRQ_HBLANK: usize = 0x1;
pub const IRQ_VCOUNTER: usize = 0x2;
pub const IRQ_TIMER0: usize = 0x3;
pub const IRQ_TIMER1: usize = 0x4;
pub const IRQ_TIMER2: usize = 0x5;
pub const IRQ_TIMER3: usize = 0x6;
pub const IRQ_SERIAL: usize = 0x7;
pub const IRQ_DMA0: usize = 0x8;
pub const IRQ_DMA1: usize = 0x9;
pub const IRQ_DMA2: usize = 0xA;
pub const IRQ_DMA3: usize = 0xB;
pub const IRQ_KEYPAD: usize = 0xC;
pub const IRQ_GAMEPAK: usize = 0xD;

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
        self.master_enable_reg.enable() && (self.request_reg.0 & self.enable_reg.0) != 0
    }

    pub fn request(&mut self, offset: usize) {
        self.request_reg.0 |= 1 << offset;
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
            // 0x202 => self.request_reg.set_lo_byte(data),
            // 0x203 => self.request_reg.set_hi_byte(data),
            0x202 => self
                .request_reg
                .set_lo_byte(self.request_reg.lo_byte() & !data),
            0x203 => self
                .request_reg
                .set_hi_byte(self.request_reg.hi_byte() & !data),

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
