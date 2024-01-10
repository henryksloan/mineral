#[macro_use]
extern crate bitfield;

mod consts;
mod dma_sound_channel;
mod registers;
mod tone_channel;
mod wave_channel;

pub use crate::registers::DmaSoundTimer;

use crate::dma_sound_channel::*;
use crate::registers::*;
use crate::tone_channel::*;
use crate::wave_channel::*;

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
    wave_channel: WaveChannel,
    dma_sound_channels: [DmaSoundChannel; 2],
    psg_left_right_reg: PsgLeftRightReg,
    dma_control_reg: DmaControlMixReg,
    master_enable: bool,
    sample_divider: u32,
    request_dma: bool,
    audio_buffer: Arc<Mutex<AudioRingBuffer>>,
}

impl SoundController {
    pub fn new(audio_buffer: Arc<Mutex<AudioRingBuffer>>) -> Self {
        Self {
            tone_channels: [ToneChannel::new(), ToneChannel::new()],
            wave_channel: WaveChannel::new(),
            dma_sound_channels: [DmaSoundChannel::new(), DmaSoundChannel::new()],
            psg_left_right_reg: PsgLeftRightReg(0),
            dma_control_reg: DmaControlMixReg(0),
            master_enable: false,
            sample_divider: 0,
            request_dma: false,
            audio_buffer,
        }
    }

    pub fn tick(&mut self) -> bool {
        for tone_channel in &mut self.tone_channels {
            tone_channel.tick();
        }

        self.wave_channel.tick();

        if self.sample_divider > 0 {
            self.sample_divider -= 1;
        } else {
            self.sample_divider = 16_777_216 / 44_100;
            let mut audio_buffer = self.audio_buffer.lock().unwrap();
            let write_i = audio_buffer.write_cursor & (audio_buffer.buffer.len() - 1);
            audio_buffer.buffer[write_i] = 0.0;
            if self.master_enable {
                let psg_multiplier = 0.25 * self.dma_control_reg.psg_vol_multiplier();

                for i in [0, 1] {
                    let psg_enabled = self.psg_left_right_reg.channel_enabled(i);
                    // TODO: Separate left and right audio
                    if !(psg_enabled.left || psg_enabled.right) {
                        continue;
                    }
                    audio_buffer.buffer[write_i] += psg_multiplier * self.tone_channels[i].sample();
                }

                let wave_enabled = self.psg_left_right_reg.channel_enabled(2);
                // TODO: Separate left and right audio
                if wave_enabled.left || wave_enabled.right {
                    println!("{}", psg_multiplier * self.wave_channel.sample());
                    audio_buffer.buffer[write_i] += psg_multiplier * self.wave_channel.sample();
                }

                for i in [0, 1] {
                    let dma_enabled = self.dma_control_reg.dma_sound_enabled(i);
                    // TODO: Separate left and right audio
                    if !(dma_enabled.left || dma_enabled.right) {
                        continue;
                    }
                    let dma_multiplier = 0.5 * self.dma_control_reg.dma_sound_vol_multiplier(i);
                    audio_buffer.buffer[write_i] +=
                        dma_multiplier * self.dma_sound_channels[i].sample();
                }
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
            0x062 => self.tone_channels[0].set_control_reg_lo(data),
            0x063 => self.tone_channels[0].set_control_reg_hi(data),
            0x064 => self.tone_channels[0].set_frequency_reg_lo(data),
            0x065 => self.tone_channels[0].set_frequency_reg_hi(data),
            0x068 => self.tone_channels[1].set_control_reg_lo(data),
            0x069 => self.tone_channels[1].set_control_reg_hi(data),
            0x06C => self.tone_channels[1].set_frequency_reg_lo(data),
            0x06D => self.tone_channels[1].set_frequency_reg_hi(data),
            0x070 => self.wave_channel.control_reg.set_lo_byte(data),
            0x071 => self.wave_channel.control_reg.set_hi_byte(data),
            0x072 => self.wave_channel.length_volume_reg.set_lo_byte(data),
            0x073 => self.wave_channel.length_volume_reg.set_hi_byte(data),
            0x074 => self.wave_channel.set_frequency_reg_lo(data),
            0x075 => self.wave_channel.set_frequency_reg_hi(data),
            0x080 => self.psg_left_right_reg.set_lo_byte(data),
            0x081 => self.psg_left_right_reg.set_hi_byte(data),
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
            0x084 => self.master_enable = (data >> 7) & 1 == 1,
            0x090..=0x09F => {
                let octet_i = addr - 0x090;
                self.wave_channel.write_pattern_octet(octet_i, data);
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
