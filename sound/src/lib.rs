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
            // buffer: vec![0.0; 512 * 16 * 2],
            buffer: vec![0.0; 512 * 16 * 8],
            write_cursor: 0,
            play_cursor: 0,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DmaSoundTimer {
    Timer0,
    Timer1,
}

pub struct SoundController {
    // TODO: DO NOT SUBMIT: Temporary implementation
    tone2_rate: u16,
    tone2_counter: u32,
    tone2_period: u32,
    sample_divider: u32,
    fifos: [VecDeque<u32>; 2],
    fifo_octet_i: [usize; 2],
    dma_timer_select: [DmaSoundTimer; 2],
    request_dma: bool,
    audio_buffer: Arc<Mutex<AudioRingBuffer>>,
}

impl SoundController {
    pub fn new(audio_buffer: Arc<Mutex<AudioRingBuffer>>) -> Self {
        Self {
            tone2_rate: 0,
            tone2_counter: 0,
            tone2_period: 262144,
            sample_divider: 0,
            fifos: [VecDeque::with_capacity(8), VecDeque::with_capacity(8)],
            fifo_octet_i: [0; 2],
            dma_timer_select: [DmaSoundTimer::Timer0; 2],
            request_dma: false,
            audio_buffer,
        }
    }

    pub fn tick(&mut self) -> bool {
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
            audio_buffer.buffer[write_i] = 0.0;
            // audio_buffer.buffer[write_i] += if self.tone2_counter < (self.tone2_period / 2) {
            //     0.25
            // } else {
            //     -0.25
            // };
            if self.fifos[0].len() > 0 {
                audio_buffer.buffer[write_i] += ((self.fifos[0].front().unwrap()
                    >> (self.fifo_octet_i[0] * 8))
                    & 0xFF) as i8 as f32
                    / 256.;
                // println!("{:?}", audio_buffer.buffer[write_i]);
            }
            if self.fifos[1].len() > 0 {
                audio_buffer.buffer[write_i] += ((self.fifos[1].front().unwrap()
                    >> (self.fifo_octet_i[1] * 8))
                    & 0xFF) as i8 as f32
                    / 256.;
            }
            audio_buffer.write_cursor += 1;
        }

        let request_dma = self.request_dma;
        self.request_dma = false;
        request_dma
    }

    pub fn on_timer_overflow(&mut self, timer: DmaSoundTimer) {
        for fifo_i in [0, 1] {
            if self.dma_timer_select[fifo_i] == timer {
                self.tick_fifo(fifo_i);
            }
        }
    }

    fn tick_fifo(&mut self, fifo_i: usize) {
        // println!("Yee");
        if self.fifo_octet_i[fifo_i] < 3 {
            self.fifo_octet_i[fifo_i] += 1;
        } else {
            self.fifos[fifo_i].pop_front();
            self.fifo_octet_i[fifo_i] = 0;
            if self.fifos[fifo_i].len() <= 4 {
                self.request_dma = true;
            }
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
            0x083 => {
                self.dma_timer_select[0] = match (data >> 2) & 1 {
                    0 => DmaSoundTimer::Timer0,
                    1 | _ => DmaSoundTimer::Timer1,
                };
                self.dma_timer_select[1] = match (data >> 6) & 1 {
                    0 => DmaSoundTimer::Timer0,
                    1 | _ => DmaSoundTimer::Timer1,
                };
                // println!("{:?}", self.dma_timer_select);
                if (data >> 3) & 1 == 1 {
                    self.fifos[0].clear();
                    self.fifo_octet_i[0] = 0;
                }
                if (data >> 7) & 1 == 1 {
                    self.fifos[1].clear();
                    self.fifo_octet_i[1] = 0;
                }
            }
            0x0A0..=0x0A7 => {
                let reg_i = addr - 0x0A0;
                let fifo_i = reg_i / 4;
                if reg_i % 4 == 0 {
                    self.fifos[fifo_i].push_back(0);
                }
                let fifo_entry = self.fifos[fifo_i].back_mut().unwrap();
                let octet_offset = (reg_i as u32 % 4) * 8;
                let octet_mask = 0xFFu32 << octet_offset;
                *fifo_entry &= !octet_mask;
                *fifo_entry |= (data as u32) << octet_offset;
            }
            _ => {}
        }
    }
}
