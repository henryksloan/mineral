use crate::Memory;

pub struct ROM<const LENGTH: usize> {
    memory: [u8; LENGTH],
}

impl<const LENGTH: usize> ROM<LENGTH> {
    pub fn new() -> Self {
        Self {
            memory: [0; LENGTH],
        }
    }

    pub fn flash(&mut self, data: [u8; LENGTH]) {
        self.memory = data;
    }
}

impl<const LENGTH: usize> Memory for ROM<LENGTH> {
    // TODO: Consider optimizing read_u16 and read_u32 using from_{le, be}_bytes
    // Same for RAM
    fn peek(&self, addr: usize) -> u8 {
        self.memory[addr]
    }

    fn write(&mut self, _addr: usize, _data: u8) {}
}
