#[macro_use]
extern crate bitfield;

mod dma_controller;
mod key_controller;

use crate::dma_controller::DmaController;
use crate::key_controller::KeyController;

use cpu::CPU;
use memory::{Memory, RAM};
use ppu::PPU;

use std::cell::RefCell;
use std::rc::Rc;

pub struct GBA {
    cpu: Rc<RefCell<CPU>>,
    ppu: Rc<RefCell<PPU>>,

    key_controller: Rc<RefCell<KeyController>>,
    dma_controller: Rc<RefCell<DmaController>>,
}

impl GBA {
    pub fn new() -> Self {
        let vram = Rc::new(RefCell::new(RAM::new()));
        let palette_ram = Rc::new(RefCell::new(RAM::new()));
        let oam = Rc::new(RefCell::new(RAM::new()));

        let ppu = Rc::new(RefCell::new(PPU::new(
            vram.clone(),
            palette_ram.clone(),
            oam.clone(),
        )));

        let key_controller = Rc::new(RefCell::new(KeyController::new()));
        let dma_controller = Rc::new(RefCell::new(DmaController::new()));

        let mmu = Rc::new(RefCell::new(MemoryMap {
            vram: vram.clone(),
            palette_ram: palette_ram.clone(),
            oam: oam.clone(),
            ppu: ppu.clone(),
            key_controller: key_controller.clone(),
            dma_controller: dma_controller.clone(),
        }));

        let cpu = Rc::new(RefCell::new(CPU::new(mmu.clone())));

        // TODO: Initialize interrupt controller

        Self {
            cpu,
            ppu,
            key_controller,
            dma_controller,
        }
    }

    pub fn tick(&mut self) {
        self.cpu.borrow_mut().tick();

        // if !dma_controller.is_active() {
        //     self.cpu.tick()
        //     if self.interrupt_controller.has_interrupt() { // IF register != 0
        //          self.cpu.irq();
        //     }
        // }
        self.ppu.borrow_mut().tick();
        // TODO: Tick APU
        // TODO: Tick timer unit, possibly alerting interrupt controller
        self.dma_controller.borrow_mut().tick(self.cpu.clone()); // TODO: Interrupts

        // TODO: When a frame is ready, the GBA should expose the framebuffer,
        // TODO: and the frontend can read it AND THEN update keypad state
    }

    pub fn try_get_framebuffer(&mut self) -> Option<[u8; 240 * 160 * 2]> {
        self.ppu.borrow_mut().try_get_framebuffer()
    }

    pub fn flash_bios(&mut self, data: Vec<u8>) {
        self.cpu.borrow_mut().flash_bios(data);
    }

    // TODO: Replace with inserting/ejecting model
    pub fn flash_cart(&mut self, data: Vec<u8>) {
        self.cpu.borrow_mut().flash_cart(data);
    }

    pub fn update_key_state(&mut self, state: u16) {
        self.key_controller.borrow_mut().set_state(state);
    }
}

struct MemoryMap {
    vram: Rc<RefCell<RAM<0x18000>>>,      // VRAM
    palette_ram: Rc<RefCell<RAM<0x400>>>, // Palette RAM
    oam: Rc<RefCell<RAM<0x400>>>,         // Object attribute memory

    ppu: Rc<RefCell<PPU>>,
    key_controller: Rc<RefCell<KeyController>>,
    dma_controller: Rc<RefCell<DmaController>>,
}

impl Memory for MemoryMap {
    fn peek(&self, addr: usize) -> u8 {
        match addr {
            0x05000000..=0x050003FF => self.palette_ram.borrow().peek(addr - 0x05000000),
            0x06000000..=0x06017FFF => self.vram.borrow().peek(addr - 0x06000000),
            0x07000000..=0x070003FF => self.oam.borrow().peek(addr - 0x07000000),

            // IO map
            0x04000000..=0x04000057 => self.ppu.borrow().peek(addr - 0x04000000),
            0x040000B0..=0x040000E1 => self.dma_controller.borrow().peek(addr - 0x04000000),
            0x04000130..=0x04000133 => self.key_controller.borrow().peek(addr - 0x04000000),
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        match addr {
            0x05000000..=0x050003FF => self.palette_ram.borrow_mut().write(addr - 0x05000000, data),
            0x06000000..=0x06017FFF => self.vram.borrow_mut().write(addr - 0x06000000, data),
            0x07000000..=0x070003FF => self.oam.borrow_mut().write(addr - 0x07000000, data),

            // IO map
            0x04000000..=0x04000057 => self.ppu.borrow_mut().write(addr - 0x04000000, data),
            0x040000B0..=0x040000E1 => self
                .dma_controller
                .borrow_mut()
                .write(addr - 0x04000000, data),
            0x04000130..=0x04000133 => self
                .key_controller
                .borrow_mut()
                .write(addr - 0x04000000, data),
            _ => {}
        }
    }
}
