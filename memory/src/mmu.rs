use std::cell::RefCell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::Memory;

/// Defines an address state made up of contiguous, inclusive ranges
pub struct MMU {
    mem_ranges: Vec<(RangeInclusive<usize>, Rc<RefCell<dyn Memory>>)>,
}

impl MMU {
    pub fn new() -> Self {
        Self {
            mem_ranges: Vec::new(),
        }
    }

    pub fn map_range(&mut self, range: RangeInclusive<usize>, memory: Rc<RefCell<dyn Memory>>) {
        self.mem_ranges.push((range, memory));
    }

    // Takes an address, and returns the mapped memory and the mapped address (if any)
    pub fn access(&self, addr: usize) -> Option<(&Rc<RefCell<dyn Memory>>, usize)> {
        self.mem_ranges
            .iter()
            .find(|&range| range.0.contains(&addr))
            .map(|range| (&range.1, addr - range.0.start()))
    }
}

impl Memory for MMU {
    fn read(&mut self, addr: usize) -> u8 {
        self.access(addr)
            .map_or(0, |range| range.0.borrow_mut().read(range.1))
    }

    fn peek(&self, addr: usize) -> u8 {
        self.access(addr)
            .map_or(0, |range| range.0.borrow().peek(range.1))
    }

    fn write(&mut self, addr: usize, data: u8) {
        self.access(addr)
            .map(|range| range.0.borrow_mut().write(range.1, data));
    }

    fn read_u16(&mut self, addr: usize) -> u16 {
        self.access(addr)
            .map_or(0, |range| range.0.borrow_mut().read_u16(range.1))
    }

    fn peek_u16(&self, addr: usize) -> u16 {
        self.access(addr)
            .map_or(0, |range| range.0.borrow().peek_u16(range.1))
    }

    fn write_u16(&mut self, addr: usize, data: u16) {
        self.access(addr)
            .map(|range| range.0.borrow_mut().write_u16(range.1, data));
    }

    fn read_u32(&mut self, addr: usize) -> u32 {
        self.access(addr)
            .map_or(0, |range| range.0.borrow_mut().read_u32(range.1))
    }

    fn peek_u32(&self, addr: usize) -> u32 {
        self.access(addr)
            .map_or(0, |range| range.0.borrow().peek_u32(range.1))
    }

    fn write_u32(&mut self, addr: usize, data: u32) {
        self.access(addr)
            .map(|range| range.0.borrow_mut().write_u32(range.1, data));
    }
}
