use crate::operating_mode::OperatingMode;

pub struct StatusRegister {
    pub raw: u32,
}

impl StatusRegister {
    pub fn new() -> Self {
        Self {
            // TODO: ??
            raw: 0x00_00_00_13, // Start in supervisor mode
        }
    }

    fn get_bit(&self, n: u8) -> bool {
        (self.raw >> n) & 1 == 1
    }

    fn set_bit(&mut self, n: u8, val: bool) {
        self.raw &= !(1 << n);
        self.raw |= (val as u32) << n;
    }

    // Condition flags: Negative, zero, carry, overflow
    pub fn get_n(&self) -> bool {
        self.get_bit(31)
    }
    pub fn get_z(&self) -> bool {
        self.get_bit(30)
    }
    pub fn get_c(&self) -> bool {
        self.get_bit(29)
    }
    pub fn get_v(&self) -> bool {
        self.get_bit(28)
    }
    pub fn set_n(&mut self, val: bool) {
        self.set_bit(31, val)
    }
    pub fn set_z(&mut self, val: bool) {
        self.set_bit(30, val)
    }
    pub fn set_c(&mut self, val: bool) {
        self.set_bit(29, val)
    }
    pub fn set_v(&mut self, val: bool) {
        self.set_bit(28, val)
    }

    // Control bits: IRQ disable, FIQ disable, Thumb mode enable
    pub fn get_i(&self) -> bool {
        self.get_bit(7)
    }
    pub fn get_f(&self) -> bool {
        self.get_bit(6)
    }
    pub fn get_t(&self) -> bool {
        self.get_bit(5)
    }
    pub fn set_i(&mut self, val: bool) {
        self.set_bit(7, val)
    }
    pub fn set_f(&mut self, val: bool) {
        self.set_bit(6, val)
    }
    pub fn set_t(&mut self, val: bool) {
        self.set_bit(5, val)
    }

    pub fn set_mode(&mut self, mode: OperatingMode) {
        self.raw &= !0b11111;
        self.raw |= mode as u32;
    }

    pub fn get_mode(&self) -> OperatingMode {
        OperatingMode::from_u32(self.raw & 0b11111).expect("Invalid operating mode")
    }
}
