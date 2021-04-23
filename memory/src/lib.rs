pub mod mmu;
pub mod ram;
pub mod rom;

pub use self::mmu::MMU;
pub use self::ram::RAM;
pub use self::rom::ROM;

pub trait Memory {
    fn read(&mut self, addr: usize) -> u8 {
        // Reads can have side-effects, but generally will be the same as peek
        self.peek(addr)
    }
    fn peek(&self, addr: usize) -> u8;
    fn write(&mut self, addr: usize, data: u8);

    fn read_u16(&mut self, addr: usize) -> u16 {
        let lo = self.read(addr) as u16;
        let hi = self.read(addr + 1) as u16;
        (hi << 8) | lo
    }

    fn peek_u16(&self, addr: usize) -> u16 {
        let lo = self.peek(addr) as u16;
        let hi = self.peek(addr + 1) as u16;
        (hi << 8) | lo
    }

    fn write_u16(&mut self, addr: usize, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.write(addr, lo);
        self.write(addr + 1, hi);
    }

    fn read_u32(&mut self, addr: usize) -> u32 {
        let lo = self.read_u16(addr) as u32;
        let hi = self.read_u16(addr + 2) as u32;
        (hi << 16) | lo
    }

    fn peek_u32(&self, addr: usize) -> u32 {
        let lo = self.peek_u16(addr) as u32;
        let hi = self.peek_u16(addr + 2) as u32;
        (hi << 16) | lo
    }

    fn write_u32(&mut self, addr: usize, data: u32) {
        let hi = (data >> 16) as u16;
        let lo = (data & 0xffff) as u16;
        self.write_u16(addr, lo);
        self.write_u16(addr + 2, hi);
    }
}
