use crate::Memory;

pub struct RAM<const LENGTH: usize> {
    memory: [u8; LENGTH],
}

impl<const LENGTH: usize> RAM<LENGTH> {
    pub fn new() -> Self {
        Self {
            memory: [0; LENGTH],
        }
    }
}

impl<const LENGTH: usize> Memory for RAM<LENGTH> {
    fn peek(&self, addr: usize) -> u8 {
        self.memory[addr]
    }

    fn write(&mut self, addr: usize, data: u8) {
        self.memory[addr] = data
    }
}
