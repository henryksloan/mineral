use crate::Memory;

pub struct ROM<const LENGTH: usize> {
    memory: Vec<u8>,
}

impl<const LENGTH: usize> ROM<LENGTH> {
    pub fn new() -> Self {
        Self {
            memory: vec![0; LENGTH],
        }
    }

    pub fn flash(&mut self, data: Vec<u8>) {
        self.memory = vec![0; LENGTH];
        self.memory[..data.len()].clone_from_slice(&data);
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
