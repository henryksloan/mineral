use crate::interrupt_controller::{self, InterruptController};

use memory::Memory;

use std::cell::RefCell;
use std::rc::Rc;

pub struct TimerController {
    control_regs: [TimerControlReg; 4],
    counters: [u16; 4],
    prescaler_counters: [u16; 4],
}

impl TimerController {
    pub fn new() -> Self {
        Self {
            control_regs: [
                TimerControlReg(0),
                TimerControlReg(0),
                TimerControlReg(0),
                TimerControlReg(0),
            ],
            counters: [0; 4],
            prescaler_counters: [0; 4],
        }
    }

    pub fn tick(&mut self, interrupt_controller: Rc<RefCell<InterruptController>>) {
        for i in 0..4 {
            if i == 0 || !self.control_regs[i].count_up() {
                self.tick_timer(i, &interrupt_controller);
            }
        }
    }

    fn tick_timer(&mut self, i: usize, interrupt_controller: &Rc<RefCell<InterruptController>>) {
        if self.control_regs[i].enable() {
            if i != 0 && self.control_regs[i].count_up() {
                self.counters[i] = self.counters[i].wrapping_add(1);
            } else {
                self.prescaler_counters[i] = self.prescaler_counters[i].wrapping_sub(1);
                if self.prescaler_counters[i] == 0 {
                    self.reload_prescaler(i);
                    self.counters[i] = self.counters[i].wrapping_sub(1);
                }
            }

            if self.counters[i] == 0 {
                self.reload_counter(i);

                if self.control_regs[i].irq() {
                    let mut irq = interrupt_controller.borrow_mut();
                    match i {
                        0 => irq.request(interrupt_controller::IRQ_TIMER0),
                        1 => irq.request(interrupt_controller::IRQ_TIMER1),
                        2 => irq.request(interrupt_controller::IRQ_TIMER2),
                        3 | _ => irq.request(interrupt_controller::IRQ_TIMER3),
                    }
                }

                if i < 3 && self.control_regs[i + 1].count_up() {
                    self.tick_timer(i + 1, interrupt_controller);
                }
            }
        }
    }

    fn set_control_byte_2(&mut self, timer_n: usize, data: u8) {
        let old_enable = self.control_regs[timer_n].enable();
        self.control_regs[timer_n].set_byte_2(data);
        let new_enable = self.control_regs[timer_n].enable();

        if !old_enable && new_enable {
            self.reload_counter(timer_n);
            self.reload_prescaler(timer_n);
        }
    }

    fn reload_counter(&mut self, timer_n: usize) {
        self.counters[timer_n] = self.control_regs[timer_n].reload();
    }

    // The number of ticks before the timer itself should be ticked
    fn reload_prescaler(&mut self, timer_n: usize) {
        self.prescaler_counters[timer_n] = match self.control_regs[timer_n].prescaler() {
            0b00 => 1,
            0b01 => 64,
            0b10 => 256,
            0b11 | _ => 1024,
        };
    }
}

impl Memory for TimerController {
    fn peek(&self, addr: usize) -> u8 {
        match addr {
            0x100 => self.counters[0] as u8,
            0x101 => (self.counters[0] >> 8) as u8,
            0x102 => self.control_regs[0].byte_2(),
            0x103 => self.control_regs[0].byte_3(),
            0x104 => self.counters[1] as u8,
            0x105 => (self.counters[1] >> 8) as u8,
            0x106 => self.control_regs[1].byte_2(),
            0x107 => self.control_regs[1].byte_3(),
            0x108 => self.counters[2] as u8,
            0x109 => (self.counters[2] >> 8) as u8,
            0x10A => self.control_regs[2].byte_2(),
            0x10B => self.control_regs[2].byte_3(),
            0x10C => self.counters[3] as u8,
            0x10D => (self.counters[3] >> 8) as u8,
            0x10E => self.control_regs[3].byte_2(),
            0x10F => self.control_regs[3].byte_3(),
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        match addr {
            0x100 => self.control_regs[0].set_byte_0(data),
            0x101 => self.control_regs[0].set_byte_1(data),
            0x102 => self.set_control_byte_2(0, data),
            0x103 => self.control_regs[0].set_byte_3(data),
            0x104 => self.control_regs[1].set_byte_0(data),
            0x105 => self.control_regs[1].set_byte_1(data),
            0x106 => self.set_control_byte_2(1, data),
            0x107 => self.control_regs[1].set_byte_3(data),
            0x108 => self.control_regs[2].set_byte_0(data),
            0x109 => self.control_regs[2].set_byte_1(data),
            0x10A => self.set_control_byte_2(2, data),
            0x10B => self.control_regs[2].set_byte_3(data),
            0x10C => self.control_regs[3].set_byte_0(data),
            0x10D => self.control_regs[3].set_byte_1(data),
            0x10E => self.set_control_byte_2(3, data),
            0x10F => self.control_regs[3].set_byte_3(data),
            _ => {}
        }
    }
}

bitfield! {
  /// 4000100h, 4000104h, 4000108h, 400010Ch - Timer Reload and Control
  /// Configures a timer
  pub struct TimerControlReg(u32);
  impl Debug;
  pub u16, reload, _: 15, 0;
  pub prescaler, _: 17, 16;
  pub count_up, _: 18;
  pub irq, _: 22;
  pub enable, _: 23;

  pub u8, byte_0, set_byte_0: 7, 0;
  pub u8, byte_1, set_byte_1: 15, 8;
  pub u8, byte_2, set_byte_2: 23, 16;
  pub u8, byte_3, set_byte_3: 31, 24;
}
