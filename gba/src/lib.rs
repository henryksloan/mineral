#[macro_use]
extern crate bitfield;

mod dma_controller;
mod interrupt_controller;
mod key_controller;
mod timer_controller;

use crate::dma_controller::DmaController;
use crate::interrupt_controller::InterruptController;
use crate::key_controller::KeyController;
use crate::timer_controller::TimerController;

use cpu::CPU;
use memory::{Memory, RAM};
use ppu::PPU;
use sound::{AudioRingBuffer, SoundController};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct GBA {
    cpu: Rc<RefCell<CPU>>,
    ppu: Rc<RefCell<PPU>>,

    sound_controller: Rc<RefCell<SoundController>>,
    key_controller: Rc<RefCell<KeyController>>,
    dma_controller: Rc<RefCell<DmaController>>,
    timer_controller: Rc<RefCell<TimerController>>,
    interrupt_controller: Rc<RefCell<InterruptController>>,

    audio_buffer: Arc<Mutex<AudioRingBuffer>>,
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

        let audio_buffer = Arc::new(Mutex::new(AudioRingBuffer::new()));

        let sound_controller = Rc::new(RefCell::new(SoundController::new(audio_buffer.clone())));
        let key_controller = Rc::new(RefCell::new(KeyController::new()));
        let dma_controller = Rc::new(RefCell::new(DmaController::new()));
        let timer_controller = Rc::new(RefCell::new(TimerController::new()));
        let interrupt_controller = Rc::new(RefCell::new(InterruptController::new()));

        let mmu = Rc::new(RefCell::new(MemoryMap {
            vram: vram.clone(),
            palette_ram: palette_ram.clone(),
            oam: oam.clone(),
            ppu: ppu.clone(),
            sound_controller: sound_controller.clone(),
            key_controller: key_controller.clone(),
            dma_controller: dma_controller.clone(),
            timer_controller: timer_controller.clone(),
            interrupt_controller: interrupt_controller.clone(),
        }));

        let cpu = Rc::new(RefCell::new(CPU::new(mmu.clone())));

        Self {
            cpu,
            ppu,
            sound_controller,
            key_controller,
            dma_controller,
            timer_controller,
            interrupt_controller,
            audio_buffer,
        }
    }

    pub fn tick(&mut self) {
        if !self.dma_controller.borrow().is_active() {
            if self.interrupt_controller.borrow().has_interrupt() {
                self.cpu.borrow_mut().irq();
            }

            self.cpu.borrow_mut().tick();
        }

        // TODO: This loop is an artificial and highly imperfect compensation for the lack of CPU
        // pipeline emulation. The imperfect timing breaks some visuals (like the 3D effect in
        // Monkey Ball Jr., I believe) and leads to sound desync.
        for _ in 0..3 {
            let (vblank, hblank, vblank_irq, hblank_irq, vcounter_irq) =
                self.ppu.borrow_mut().tick();

            if vblank {
                self.dma_controller.borrow_mut().on_vblank();
            }
            if hblank {
                self.dma_controller.borrow_mut().on_hblank();
            }

            if vblank_irq {
                self.interrupt_controller
                    .borrow_mut()
                    .request(interrupt_controller::IRQ_VBLANK);
            }
            if hblank_irq {
                self.interrupt_controller
                    .borrow_mut()
                    .request(interrupt_controller::IRQ_HBLANK);
            }
            if vcounter_irq {
                self.interrupt_controller
                    .borrow_mut()
                    .request(interrupt_controller::IRQ_VCOUNTER);
            }

            let sound_dma_requested = self.sound_controller.borrow_mut().tick();
            if sound_dma_requested {
                self.dma_controller.borrow_mut().on_dma_sound_request();
            }
            self.timer_controller.borrow_mut().tick(
                self.interrupt_controller.clone(),
                self.sound_controller.clone(),
            );
            self.dma_controller
                .borrow_mut()
                .tick(self.cpu.clone(), self.interrupt_controller.clone());
        }

        // TODO: When a frame is ready, the GBA should expose the framebuffer,
        // TODO: and the frontend can read it AND THEN update keypad state
    }

    pub fn try_get_framebuffer(&mut self) -> Option<[u8; 240 * 160 * 2]> {
        self.ppu.borrow_mut().try_get_framebuffer()
    }

    pub fn get_audio_buffer(&self) -> Arc<Mutex<AudioRingBuffer>> {
        self.audio_buffer.clone()
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
    sound_controller: Rc<RefCell<SoundController>>,
    key_controller: Rc<RefCell<KeyController>>,
    dma_controller: Rc<RefCell<DmaController>>,
    timer_controller: Rc<RefCell<TimerController>>,
    interrupt_controller: Rc<RefCell<InterruptController>>,
}

impl Memory for MemoryMap {
    fn peek(&self, addr: usize) -> u8 {
        match addr {
            0x05000000..=0x05FFFFFF => self.palette_ram.borrow().peek((addr - 0x05000000) % 0x400),
            0x06000000..=0x06FFFFFF => self
                .vram
                .borrow()
                .peek(((addr - 0x06000000) % 0x20000) % 0x18000),
            0x07000000..=0x07FFFFFF => self.oam.borrow().peek((addr - 0x07000000) % 0x400),

            // IO map
            0x04000000..=0x04000057 => self.ppu.borrow().peek(addr - 0x04000000),
            0x04000060..=0x040000A8 => self.sound_controller.borrow().peek(addr - 0x04000000),
            0x040000B0..=0x040000E1 => self.dma_controller.borrow().peek(addr - 0x04000000),
            0x04000100..=0x04000111 => self.timer_controller.borrow().peek(addr - 0x04000000),
            0x04000130..=0x04000133 => self.key_controller.borrow().peek(addr - 0x04000000),
            0x04000200..=0x0400020B => self.interrupt_controller.borrow().peek(addr - 0x04000000),
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        match addr {
            0x05000000..=0x05FFFFFF => self
                .palette_ram
                .borrow_mut()
                .write((addr - 0x05000000) % 0x400, data),
            0x06000000..=0x06FFFFFF => self
                .vram
                .borrow_mut()
                .write(((addr - 0x06000000) % 0x20000) % 0x18000, data),
            0x07000000..=0x07FFFFFF => self
                .oam
                .borrow_mut()
                .write((addr - 0x07000000) % 0x400, data),

            // IO map
            0x04000000..=0x04000057 => self.ppu.borrow_mut().write(addr - 0x04000000, data),
            0x04000060..=0x040000A8 => self
                .sound_controller
                .borrow_mut()
                .write(addr - 0x04000000, data),
            0x040000B0..=0x040000E1 => self
                .dma_controller
                .borrow_mut()
                .write(addr - 0x04000000, data),
            0x04000100..=0x04000111 => self
                .timer_controller
                .borrow_mut()
                .write(addr - 0x04000000, data),
            0x04000130..=0x04000133 => self
                .key_controller
                .borrow_mut()
                .write(addr - 0x04000000, data),
            0x04000200..=0x0400020B => self
                .interrupt_controller
                .borrow_mut()
                .write(addr - 0x04000000, data),
            _ => {}
        }
    }
}
