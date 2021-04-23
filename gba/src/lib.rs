mod io_controller;

use crate::io_controller::IoController;

use cpu::CPU;
use memory::{MMU, RAM, ROM};
use ppu::PPU;

use std::cell::RefCell;
use std::rc::Rc;

pub struct GBA {
    cpu: Rc<RefCell<CPU>>,
    ppu: Rc<RefCell<PPU>>,

    bios_rom: Rc<RefCell<ROM<0x4000>>>,   // BIOS ROM
    ewram: Rc<RefCell<RAM<0x40000>>>,     // Internal work RAM
    iwram: Rc<RefCell<RAM<0x8000>>>,      // External work RAM
    vram: Rc<RefCell<RAM<0x18000>>>,      // VRAM
    palette_ram: Rc<RefCell<RAM<0x400>>>, // Palette RAM
    oam: Rc<RefCell<RAM<0x400>>>,         // Object attribute memory
    cart_rom: Rc<RefCell<ROM<0x400000>>>, // Cartridge ROM

    mmu: Rc<RefCell<MMU>>, // Defines the CPU/DMA address space
    io_controller: Rc<RefCell<IoController>>,
}

impl GBA {
    pub fn new() -> Self {
        let bios_rom = Rc::new(RefCell::new(ROM::new()));
        let ewram = Rc::new(RefCell::new(RAM::new()));
        let iwram = Rc::new(RefCell::new(RAM::new()));
        let vram = Rc::new(RefCell::new(RAM::new()));
        let palette_ram = Rc::new(RefCell::new(RAM::new()));
        let oam = Rc::new(RefCell::new(RAM::new()));
        let cart_rom = Rc::new(RefCell::new(ROM::new()));

        let mmu = Rc::new(RefCell::new(MMU::new()));

        // TODO: Initialize the DMA controller with the MMU Rc

        let cpu = Rc::new(RefCell::new(CPU::new())); // TODO: Pass a clone of the MMU Rc to CPU
        let ppu = Rc::new(RefCell::new(PPU::new(
            vram.clone(),
            palette_ram.clone(),
            oam.clone(),
        )));

        let io_controller = Rc::new(RefCell::new(IoController::new(ppu.clone())));
        // TODO: Initialize interrupt controller

        // Populate the address space
        {
            let mut mmu_mut = mmu.borrow_mut();
            mmu_mut.map_range(0x00000000..=0x00003FFF, bios_rom.clone());
            mmu_mut.map_range(0x02000000..=0x0203FFFF, ewram.clone());
            mmu_mut.map_range(0x03000000..=0x0307FFFF, iwram.clone());
            mmu_mut.map_range(0x04000000..=0x040003FE, io_controller.clone());
            mmu_mut.map_range(0x05000000..=0x050003FF, palette_ram.clone());
            mmu_mut.map_range(0x06000000..=0x06017FFF, vram.clone());
            mmu_mut.map_range(0x08000000..=0x0DFFFFFF, cart_rom.clone());
        }

        Self {
            cpu,
            ppu,

            bios_rom,
            ewram,
            iwram,
            vram,
            palette_ram,
            oam,
            cart_rom,

            mmu,
            io_controller,
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
        // TODO: ppu.tick()
        // TODO: Tick APU
        // TODO: Tick timer unit, possibly alerting interrupt controller
        // TODO: Tick DMA controller which should make some copies and possibly alert interrupt
        // TODO: When a frame is ready, the GBA should expose the framebuffer,
        // TODO: and the frontend can read it AND THEN update keypad state
    }
}
