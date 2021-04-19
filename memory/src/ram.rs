use crate::Memory;

pub struct RAM<const LENGTH: usize> {
    memory: Vec<u8>,
}

impl<const LENGTH: usize> RAM<LENGTH> {
    pub fn new() -> Self {
        Self {
            memory: vec![0; LENGTH],
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
