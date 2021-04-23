use memory::Memory;
use ppu::PPU;

use std::cell::RefCell;
use std::rc::Rc;

pub struct IoController {
    ppu: Rc<RefCell<PPU>>,
}

impl IoController {
    pub fn new(ppu: Rc<RefCell<PPU>>) -> Self {
        Self { ppu }
    }
}

impl Memory for IoController {
    fn peek(&self, addr: usize) -> u8 {
        todo!()
    }

    fn write(&mut self, addr: usize, data: u8) {
        todo!()
    }
}
