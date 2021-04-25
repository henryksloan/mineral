use memory::Memory;

pub struct KeyController {
    state: u16,
}

impl KeyController {
    pub fn new() -> Self {
        Self { state: 0xFF }
    }

    pub fn set_state(&mut self, data: u16) {
        self.state = data;
    }
}

impl Memory for KeyController {
    fn peek(&self, addr: usize) -> u8 {
        // TODO: Key Interrupt Control
        if addr == 0x130 {
            (self.state & 0xFF) as u8
        } else if addr == 0x131 {
            ((self.state >> 8) & 0xFF) as u8
        } else {
            0
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        // TODO
    }
}
