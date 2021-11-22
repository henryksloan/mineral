use crate::interrupt_controller::{self, InterruptController};

use memory::Memory;

use std::cell::RefCell;
use std::rc::Rc;

pub struct DmaController {
    source_regs: [DmaAddressReg; 4],
    dest_regs: [DmaAddressReg; 4],

    control_regs: [DmaControlReg; 4],

    transfers_active: [bool; 4],
    // Internal control registers and source/dest addresses of active channels
    transfers: [(DmaControlReg, usize, usize); 4],
}

impl DmaController {
    pub fn new() -> Self {
        Self {
            source_regs: [
                DmaAddressReg(0),
                DmaAddressReg(0),
                DmaAddressReg(0),
                DmaAddressReg(0),
            ],
            dest_regs: [
                DmaAddressReg(0),
                DmaAddressReg(0),
                DmaAddressReg(0),
                DmaAddressReg(0),
            ],

            control_regs: [
                DmaControlReg(0),
                DmaControlReg(0),
                DmaControlReg(0),
                DmaControlReg(0),
            ],

            transfers_active: [false; 4],
            transfers: [
                (DmaControlReg(0), 0, 0),
                (DmaControlReg(0), 0, 0),
                (DmaControlReg(0), 0, 0),
                (DmaControlReg(0), 0, 0),
            ],
        }
    }

    pub fn is_active(&self) -> bool {
        self.transfers_active.iter().any(|&active| active)
    }

    pub fn tick(
        &mut self,
        memory_rc: Rc<RefCell<dyn Memory>>,
        interrupt_controller: Rc<RefCell<InterruptController>>,
    ) {
        // TODO: Implement accurate transfer timing
        let mut memory = memory_rc.borrow_mut();
        for channel in 0..4 {
            if self.transfers_active[channel] {
                let active_transfer = &mut self.transfers[channel];
                let mut n_units = match active_transfer.0.n_units() {
                    0 => {
                        if channel == 3 {
                            0x10000
                        } else {
                            0x4000
                        }
                    }
                    n_units => n_units,
                };
                if channel == 3 {
                    n_units = std::cmp::min(n_units, 0x10000);
                } else {
                    n_units = std::cmp::min(n_units, 0x4000);
                }
                for _ in 0..n_units {
                    let unit_size = if active_transfer.0.unit_size() { 4 } else { 2 };

                    if !((0x040000B0..=0x040000E1).contains(&active_transfer.1))
                        && !((0x040000B0..=0x040000E1).contains(&active_transfer.2))
                    {
                        if unit_size == 4 {
                            let data = memory.read_u32(active_transfer.1 & !0b11);
                            memory.write_u32(active_transfer.2 & !0b11, data);
                        } else {
                            let data = memory.read_u16(active_transfer.1 & !0b1);
                            memory.write_u16(active_transfer.2 & !0b1, data);
                        }
                    }

                    match active_transfer.0.dest_adjustment() {
                        0b00 | 0b11 => {
                            active_transfer.2 = active_transfer.2.wrapping_add(unit_size)
                        }
                        0b01 => active_transfer.2 = active_transfer.2.wrapping_sub(unit_size),
                        0b10 => {} // Fixed
                        _ => {}    // TODO
                    }

                    match active_transfer.0.source_adjustment() {
                        0b00 => active_transfer.1 = active_transfer.1.wrapping_add(unit_size),
                        0b01 => active_transfer.1 = active_transfer.1.wrapping_sub(unit_size),
                        0b10 => {} // Fixed
                        0b11 | _ => panic!("illegal DMA source adjustment mode"),
                    }
                }

                if active_transfer.0.irq() {
                    let mut irq = interrupt_controller.borrow_mut();
                    match channel {
                        0 => irq.request(interrupt_controller::IRQ_DMA0),
                        1 => irq.request(interrupt_controller::IRQ_DMA1),
                        2 => irq.request(interrupt_controller::IRQ_DMA2),
                        3 | _ => irq.request(interrupt_controller::IRQ_DMA3),
                    }
                }

                self.transfers_active[channel] = false;
                if !self.control_regs[channel].repeat() {
                    self.control_regs[channel].set_enable(false);
                }

                break;
            }
        }
    }

    pub fn on_hblank(&mut self) {
        for channel_n in 0..4 {
            let enable = self.control_regs[channel_n].enable();
            let start_timing = self.control_regs[channel_n].start_timing();
            if enable && start_timing == 0b10 {
                self.activate_channel(channel_n, true);
            }
        }
    }

    pub fn on_vblank(&mut self) {
        for channel_n in 0..4 {
            let enable = self.control_regs[channel_n].enable();
            let start_timing = self.control_regs[channel_n].start_timing();
            if enable && start_timing == 0b01 {
                self.activate_channel(channel_n, true);
            }
        }
    }

    fn activate_channel(&mut self, channel_n: usize, repeat: bool) {
        self.transfers[channel_n].0 = DmaControlReg(self.control_regs[channel_n].0);
        if !repeat {
            let source = if channel_n == 0 {
                self.source_regs[channel_n].internal_addr_bits()
            } else {
                self.source_regs[channel_n].external_addr_bits()
            };
            self.transfers[channel_n].1 = source as usize;
        }
        if !repeat || (self.control_regs[channel_n].dest_adjustment() == 0b11) {
            let dest = if channel_n == 3 {
                self.dest_regs[channel_n].external_addr_bits()
            } else {
                self.dest_regs[channel_n].internal_addr_bits()
            };
            self.transfers[channel_n].2 = dest as usize;
        }

        self.transfers_active[channel_n] = true;
    }

    fn set_control_high_byte(&mut self, channel_n: usize, data: u8) {
        let old_enable = self.control_regs[channel_n].enable();
        self.control_regs[channel_n].set_byte_3(data);

        let start_timing = self.control_regs[channel_n].start_timing();
        let new_enable = self.control_regs[channel_n].enable();
        if start_timing == 0 && !old_enable && new_enable {
            self.activate_channel(channel_n, false);
        }
    }
}

impl Memory for DmaController {
    fn peek(&self, addr: usize) -> u8 {
        match addr {
            // Channel 0
            0x0BA => self.control_regs[0].byte_2(),
            0x0BB => self.control_regs[0].byte_3(),

            // Channel 1
            0x0C6 => self.control_regs[1].byte_2(),
            0x0C7 => self.control_regs[1].byte_3(),

            // Channel 2
            0x0D2 => self.control_regs[2].byte_2(),
            0x0D3 => self.control_regs[2].byte_3(),

            // Channel 3
            0x0DE => self.control_regs[3].byte_2(),
            0x0DF => self.control_regs[3].byte_3(),
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        match addr {
            // Channel 0
            0x0B0 => self.source_regs[0].set_byte_0(data),
            0x0B1 => self.source_regs[0].set_byte_1(data),
            0x0B2 => self.source_regs[0].set_byte_2(data),
            0x0B3 => self.source_regs[0].set_byte_3(data),
            0x0B4 => self.dest_regs[0].set_byte_0(data),
            0x0B5 => self.dest_regs[0].set_byte_1(data),
            0x0B6 => self.dest_regs[0].set_byte_2(data),
            0x0B7 => self.dest_regs[0].set_byte_3(data),
            0x0B8 => self.control_regs[0].set_byte_0(data),
            0x0B9 => self.control_regs[0].set_byte_1(data),
            0x0BA => self.control_regs[0].set_byte_2(data),
            0x0BB => self.set_control_high_byte(0, data),

            // Channel 1
            0x0BC => self.source_regs[1].set_byte_0(data),
            0x0BD => self.source_regs[1].set_byte_1(data),
            0x0BE => self.source_regs[1].set_byte_2(data),
            0x0BF => self.source_regs[1].set_byte_3(data),
            0x0C0 => self.dest_regs[1].set_byte_0(data),
            0x0C1 => self.dest_regs[1].set_byte_1(data),
            0x0C2 => self.dest_regs[1].set_byte_2(data),
            0x0C3 => self.dest_regs[1].set_byte_3(data),
            0x0C4 => self.control_regs[1].set_byte_0(data),
            0x0C5 => self.control_regs[1].set_byte_1(data),
            0x0C6 => self.control_regs[1].set_byte_2(data),
            0x0C7 => self.set_control_high_byte(1, data),

            // Channel 2
            0x0C8 => self.source_regs[2].set_byte_0(data),
            0x0C9 => self.source_regs[2].set_byte_1(data),
            0x0CA => self.source_regs[2].set_byte_2(data),
            0x0CB => self.source_regs[2].set_byte_3(data),
            0x0CC => self.dest_regs[2].set_byte_0(data),
            0x0CD => self.dest_regs[2].set_byte_1(data),
            0x0CE => self.dest_regs[2].set_byte_2(data),
            0x0CF => self.dest_regs[2].set_byte_3(data),
            0x0D0 => self.control_regs[2].set_byte_0(data),
            0x0D1 => self.control_regs[2].set_byte_1(data),
            0x0D2 => self.control_regs[2].set_byte_2(data),
            0x0D3 => self.set_control_high_byte(2, data),

            // Channel 3
            0x0D4 => self.source_regs[3].set_byte_0(data),
            0x0D5 => self.source_regs[3].set_byte_1(data),
            0x0D6 => self.source_regs[3].set_byte_2(data),
            0x0D7 => self.source_regs[3].set_byte_3(data),
            0x0D8 => self.dest_regs[3].set_byte_0(data),
            0x0D9 => self.dest_regs[3].set_byte_1(data),
            0x0DA => self.dest_regs[3].set_byte_2(data),
            0x0DB => self.dest_regs[3].set_byte_3(data),
            0x0DC => self.control_regs[3].set_byte_0(data),
            0x0DD => self.control_regs[3].set_byte_1(data),
            0x0DE => self.control_regs[3].set_byte_2(data),
            0x0DF => self.set_control_high_byte(3, data),
            _ => {}
        }
    }
}

bitfield! {
  /// 40000B0h, 40000BCh, 40000C8h, 40000D4h - DMA{0,1,2,3}SAD
  /// 40000B4h, 40000C0h, 40000CCh, 40000D8h - DMA{0,1,2,3}DAD
  /// Sets the source or destination address of a DMA channel
  pub struct DmaAddressReg(u32);
  impl Debug;
  pub internal_addr_bits, _: 26, 0;
  pub external_addr_bits, _: 27, 0;

  pub u8, byte_0, set_byte_0: 7, 0;
  pub u8, byte_1, set_byte_1: 15, 8;
  pub u8, byte_2, set_byte_2: 23, 16;
  pub u8, byte_3, set_byte_3: 31, 24;
}

bitfield! {
  /// 40000B8h, 40000C4h, 40000D0h, 40000DCh - DMA{0,1,2,3}CNT
  /// Configures and controls a DMA channel
  pub struct DmaControlReg(u32);
  impl Debug;
  pub n_units, set_n_units: 15, 0;
  pub dest_adjustment, _: 22, 21;
  pub source_adjustment, _: 24, 23;
  pub repeat, _: 25;
  pub unit_size, _: 26;
  pub start_timing, _: 29, 28;
  pub irq, _: 30;
  pub enable, set_enable: 31;

  pub u8, byte_0, set_byte_0: 7, 0;
  pub u8, byte_1, set_byte_1: 15, 8;
  pub u8, byte_2, set_byte_2: 23, 16;
  pub u8, byte_3, set_byte_3: 31, 24;
}
