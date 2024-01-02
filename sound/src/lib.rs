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
    // TODO: DO NOT SUBMIT: Temporary implementation
    tone2_rate: u16,
    tone2_counter: u32,
    tone2_period: u32,
    sample_divider: u32,
    audio_buffer: Arc<Mutex<AudioRingBuffer>>,
}

impl SoundController {
    pub fn new(audio_buffer: Arc<Mutex<AudioRingBuffer>>) -> Self {
        Self {
            tone2_rate: 0,
            tone2_counter: 0,
            tone2_period: 262144,
            sample_divider: 0,
            audio_buffer,
        }
    }

    pub fn tick(&mut self) {
        if self.tone2_counter > 0 {
            self.tone2_counter -= 1;
        } else {
            self.tone2_counter = self.tone2_period;
        }

        if self.sample_divider > 0 {
            self.sample_divider -= 1;
        } else {
            self.sample_divider = 16_777_216 / 44_100;
            let mut audio_buffer = self.audio_buffer.lock().unwrap();
            let write_i = audio_buffer.write_cursor & (audio_buffer.buffer.len() - 1);
            audio_buffer.buffer[write_i] = if self.tone2_counter < (self.tone2_period / 2) {
                0.5
            } else {
                -0.5
            };
            audio_buffer.write_cursor += 1;
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
            0x06C => {
                self.tone2_rate &= 0xFF00;
                self.tone2_rate |= data as u16;
                self.tone2_period = 16_777_216 / (131072 / (2048 - self.tone2_rate as u32));
            }
            0x06D => {
                self.tone2_rate &= 0x00FF;
                self.tone2_rate |= (data as u16 & 0b111) << 8;
                self.tone2_period = 16_777_216 / (131072 / (2048 - self.tone2_rate as u32));
            }
            _ => {}
        }
    }
}
