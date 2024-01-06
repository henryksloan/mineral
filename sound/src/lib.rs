#[macro_use]
extern crate bitfield;

mod dma_sound_channel;
mod registers;
mod tone_channel;

pub use crate::registers::DmaSoundTimer;

use crate::dma_sound_channel::*;
use crate::registers::*;
use crate::tone_channel::*;

use memory::Memory;

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

pub struct SoundController {
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
        // TODO: DO NOT SUBMIT: Implement reading
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
