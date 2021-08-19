use crate::InterruptController;

use memory::Memory;

use std::cell::RefCell;
use std::rc::Rc;

pub struct DmaController {
    source_regs: [DmaAddressReg; 4],
    dest_regs: [DmaAddressReg; 4],

    control_regs: [DmaControlReg; 4],

    // Internal control registers and source/dest addresses of active channels
    active_transfers: [Option<(DmaControlReg, usize, usize)>; 4],
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

            active_transfers: [None, None, None, None],
        }
    }

    pub fn is_active(&self) -> bool {
        self.active_transfers
            .iter()
            .any(|transfer| transfer.is_some())
    }

    pub fn tick(
        &mut self,
        memory_rc: Rc<RefCell<dyn Memory>>,
        interrupt_controller: Rc<RefCell<InterruptController>>,
    ) {
        // TODO: HBlank, VBlank timing
        // TODO: Implement accurate transfer timing
        let mut memory = memory_rc.borrow_mut();
        for channel in 0..4 {
            if let Some(active_transfer) = &mut self.active_transfers[channel] {
                for _ in 0..active_transfer.0.n_units() as usize {
                    let unit_size = if active_transfer.0.unit_size() { 4 } else { 2 };

                    if unit_size == 4 {
                        let data = memory.read_u32(active_transfer.1);
                        memory.write_u32(active_transfer.2, data);
                    } else {
                        let data = memory.read_u16(active_transfer.1);
                        memory.write_u16(active_transfer.2, data);
                    }

                    match active_transfer.0.dest_adjustment() {
                        0b00 => active_transfer.2 = active_transfer.2.wrapping_add(unit_size),
                        0b01 => active_transfer.2 = active_transfer.2.wrapping_sub(unit_size),
                        0b10 => {}     // Fixed
                        0b11 | _ => {} // TODO
                    }

                    match active_transfer.0.source_adjustment() {
                        0b00 => active_transfer.1 = active_transfer.1.wrapping_add(unit_size),
                        0b01 => active_transfer.1 = active_transfer.1.wrapping_sub(unit_size),
                        0b10 => {} // Fixed
                        0b11 | _ => panic!("illegal DMA source adjustment mode"),
                    }
                }
                self.active_transfers[channel] = None;
                self.control_regs[channel].set_enable(false);
                if self.control_regs[channel].irq() {
                    let mut irq = interrupt_controller.borrow_mut();
                    match channel {
                        0 => irq.request_reg.set_dma0(true),
                        1 => irq.request_reg.set_dma1(true),
                        2 => irq.request_reg.set_dma2(true),
                        3 | _ => irq.request_reg.set_dma3(true),
                    }
                }

                break;
            }
        }
    }

    fn set_control_high_byte(&mut self, channel_n: usize, data: u8) {
        let old_enable = self.control_regs[channel_n].enable();
        self.control_regs[channel_n].set_byte_3(data);

        let start_timing = self.control_regs[channel_n].start_timing();
        if start_timing == 0 {
            let new_enable = self.control_regs[channel_n].enable();
            if !old_enable && new_enable {
                self.active_transfers[channel_n] = Some((
                    DmaControlReg(self.control_regs[channel_n].0),
                    self.source_regs[channel_n].external_addr_bits() as usize,
                    self.dest_regs[channel_n].external_addr_bits() as usize,
                ));
            }
        }
    }
}

impl Memory for DmaController {
    fn peek(&self, addr: usize) -> u8 {
        match addr {
            // Channel 0
            0x0B8 => self.control_regs[0].byte_0(),
            0x0B9 => self.control_regs[0].byte_1(),
            0x0BA => self.control_regs[0].byte_2(),
            0x0BB => self.control_regs[0].byte_3(),

            // Channel 1
            0x0C4 => self.control_regs[1].byte_0(),
            0x0C5 => self.control_regs[1].byte_1(),
            0x0C6 => self.control_regs[1].byte_2(),
            0x0C7 => self.control_regs[1].byte_3(),

            // Channel 2
            0x0D0 => self.control_regs[2].byte_0(),
            0x0D1 => self.control_regs[2].byte_1(),
            0x0D2 => self.control_regs[2].byte_2(),
            0x0D3 => self.control_regs[2].byte_3(),

            // Channel 3
            0x0DC => self.control_regs[3].byte_0(),
            0x0DD => self.control_regs[3].byte_1(),
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
  pub n_units, _: 15, 0;
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
