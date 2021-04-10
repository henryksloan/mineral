#[derive(PartialEq)]
pub enum OperatingMode {
    User = 0b10000,
    FastInterrupt = 0b10001,
    Interrupt = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    System = 0b11111,
    Undefined = 0b11011,
}

impl OperatingMode {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            0b10000 => Some(Self::User),
            0b10001 => Some(Self::FastInterrupt),
            0b10010 => Some(Self::Interrupt),
            0b10011 => Some(Self::Supervisor),
            0b10111 => Some(Self::Abort),
            0b11011 => Some(Self::Undefined),
            0b11111 => Some(Self::System),
            _ => None,
        }
    }
}
