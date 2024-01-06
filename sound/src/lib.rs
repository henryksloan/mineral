#[macro_use]
extern crate bitfield;

mod registers;
mod tone_channel;

pub use crate::registers::DmaSoundTimer;

use crate::registers::*;
use crate::tone_channel::*;

use memory::Memory;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub struct AudioRingBuffer {
    pub buffer: Vec<f32>,
    pub write_cursor: usize,
    pub play_cursor: usize,
}

impl AudioRingBuffer {
    pub fn new() -> Self {
        Self {
            buffer: vec![0.0; 512 * 16 * 2],
            write_cursor: 0,
            play_cursor: 0,
        }
    }
}

pub struct DmaSoundChannel {
    fifo: VecDeque<u32>,
    fifo_octet_i: usize,
}

impl DmaSoundChannel {
    pub fn new() -> Self {
        Self {
            fifo: VecDeque::with_capacity(8),
            fifo_octet_i: 0,
        }
    }

    // Handles a write to one of the FIFO data registers. A write to the 0th octet pushes a new
    // entry into the FIFO, while writes to the other offset simply overwrite octets in the most
    // recent entry. I'm not sure if this is realistic, but I also don't think out-of-order writes
    // to the 8-bit FIFO registers are well-defined.
    pub fn write_fifo_octet(&mut self, octet_i: usize, data: u8) {
        if octet_i == 0 {
            self.fifo.push_back(0);
        }
        let fifo_entry = self.fifo.back_mut().unwrap();
        let octet_offset = octet_i * 8;
        let octet_mask = 0xFFu32 << octet_offset;
        *fifo_entry &= !octet_mask;
        *fifo_entry |= (data as u32) << octet_offset;
    }

    /// Returns true if a DMA is to be requested
    pub fn tick_fifo(&mut self) -> bool {
        if self.fifo_octet_i < 3 {
            self.fifo_octet_i += 1;
            false
        } else {
            self.fifo.pop_front();
            self.fifo_octet_i = 0;
            self.fifo.len() <= 4
        }
    }

    pub fn sample(&self) -> f32 {
        if let Some(val) = self.fifo.front() {
            ((val >> (self.fifo_octet_i * 8)) & 0xFF) as i8 as f32 / 128.
        } else {
            0.0
        }
    }

    fn restart(&mut self) {
        self.fifo.clear();
        self.fifo_octet_i = 0;
    }
}

pub struct SoundController {
    // TODO: DO NOT SUBMIT: Temporary implementation
    tone_channels: [ToneChannel; 2],
    dma_sound_channels: [DmaSoundChannel; 2],
    dma_control_reg: DmaControlMixReg,
    sample_divider: u32,
    request_dma: bool,
    audio_buffer: Arc<Mutex<AudioRingBuffer>>,
}

impl SoundController {
    pub fn new(audio_buffer: Arc<Mutex<AudioRingBuffer>>) -> Self {
        Self {
            tone_channels: [ToneChannel::new(), ToneChannel::new()],
            dma_sound_channels: [DmaSoundChannel::new(), DmaSoundChannel::new()],
            dma_control_reg: DmaControlMixReg(0),
            sample_divider: 0,
            request_dma: false,
            audio_buffer,
        }
    }

    pub fn tick(&mut self) -> bool {
        for tone_channel in &mut self.tone_channels {
            tone_channel.tick();
        }

        if self.sample_divider > 0 {
            self.sample_divider -= 1;
        } else {
            self.sample_divider = 16_777_216 / 44_100;
            let mut audio_buffer = self.audio_buffer.lock().unwrap();
            let write_i = audio_buffer.write_cursor & (audio_buffer.buffer.len() - 1);
            audio_buffer.buffer[write_i] = 0.0;
            for tone_channel in &self.tone_channels {
                audio_buffer.buffer[write_i] += 0.25 * tone_channel.sample();
            }
            for dma_sound_channel in &self.dma_sound_channels {
                audio_buffer.buffer[write_i] += 0.5 * dma_sound_channel.sample();
            }
            audio_buffer.write_cursor += 1;
        }

        let request_dma = self.request_dma;
        self.request_dma = false;
        request_dma
    }

    pub fn on_timer_overflow(&mut self, timer: DmaSoundTimer) {
        if self.dma_control_reg.dma_a_timer() == timer {
            self.request_dma |= self.dma_sound_channels[0].tick_fifo();
        }
        if self.dma_control_reg.dma_b_timer() == timer {
            self.request_dma |= self.dma_sound_channels[1].tick_fifo();
        }
    }
}

impl Memory for SoundController {
    fn peek(&self, addr: usize) -> u8 {
        match addr {
            _ => 0,
        }
    }

    fn write(&mut self, addr: usize, data: u8) {
        match addr {
            0x060 => self.tone_channels[0].sweep_reg.set_lo_byte(data),
            0x061 => self.tone_channels[0].sweep_reg.set_hi_byte(data),
            0x062 => self.tone_channels[0].control_reg.set_lo_byte(data),
            0x063 => self.tone_channels[0].control_reg.set_hi_byte(data),
            0x064 => self.tone_channels[0].set_frequency_reg_lo(data),
            0x065 => self.tone_channels[0].set_frequency_reg_hi(data),
            0x068 => self.tone_channels[0].control_reg.set_lo_byte(data),
            0x069 => self.tone_channels[0].control_reg.set_hi_byte(data),
            0x06C => self.tone_channels[0].set_frequency_reg_lo(data),
            0x06D => self.tone_channels[0].set_frequency_reg_hi(data),
            0x082 => self.dma_control_reg.set_lo_byte(data),
            0x083 => {
                self.dma_control_reg.set_hi_byte(data);
                if self.dma_control_reg.dma_a_restart() {
                    self.dma_sound_channels[0].restart();
                }
                if self.dma_control_reg.dma_b_restart() {
                    self.dma_sound_channels[1].restart();
                }
            }
            0x0A0..=0x0A7 => {
                let (fifo_i, octet_i) = {
                    let reg_i = addr - 0x0A0;
                    (reg_i / 4, reg_i % 4)
                };
                self.dma_sound_channels[fifo_i].write_fifo_octet(octet_i, data);
            }
            _ => {}
        }
    }
}
