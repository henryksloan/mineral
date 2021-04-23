use std::cell::RefCell;
use std::rc::Rc;

use memory::Memory;

pub struct PPU {
    vram: Rc<RefCell<dyn Memory>>,
    palette_ram: Rc<RefCell<dyn Memory>>,
    oam: Rc<RefCell<dyn Memory>>,

    scan_line: u8,
    scan_cycle: u32,

    bg_mode: u16, // TODO: Enum

    framebuffer: [u8; 240 * 160 * 2],
}

impl PPU {
    pub fn new(
        vram: Rc<RefCell<dyn Memory>>,
        palette_ram: Rc<RefCell<dyn Memory>>,
        oam: Rc<RefCell<dyn Memory>>,
    ) -> Self {
        Self {
            vram,
            palette_ram,
            oam,

            scan_line: 0,
            scan_cycle: 0,

            bg_mode: 0,

            framebuffer: [0; 240 * 160 * 2],
        }
    }

    pub fn tick(&mut self) {
        if self.scan_cycle == 0 {
            self.draw_scanline();
        }
        self.increment_scan();
    }

    pub fn vcount(&self) -> u8 {
        self.scan_line
    }

    fn increment_scan(&mut self) {
        self.scan_cycle = (self.scan_cycle + 1) % 1232;

        if self.scan_cycle == 0 {
            self.scan_line = (self.scan_line + 1) % 228;

            if self.scan_line == 160 {
                // TODO: VBlank interrupt if enabled
            }

            // TODO: V-Counter interrupt if enabled
        } else if self.scan_cycle == 960 {
            // TODO: HBlank interrupt if enabled
        }
    }

    fn draw_scanline(&mut self) {
        // TODO: Should generate a line from each active layer (backgrounds, obj),
        // taking into account windowing, then combine them based on priority and blending
        // match self.bg_mode {
        //     0 => {
        //
        //     }
        // }
    }
}
