#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SweepDirection {
    Decrease,
    Increase,
}

impl From<u8> for SweepDirection {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Increase,
            1 | _ => Self::Decrease,
        }
    }
}

bitfield! {
  /// 4000060h - SOUND1CNT_L
  /// Configures tone sweep for channel 1
  pub struct ToneSweepReg(u16);
  impl Debug;
  pub sweep_shift_n, _: 2, 0;
  // bitfield ignores types and `into` for single-index fields, so `3, 3` tells it to treat it
  // like a non-bool field.
  pub u8, into SweepDirection, sweep_dir, _: 3, 3;
  pub sweep_time, _: 6, 4;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EnvelopeDirection {
    Decrease,
    Increase,
}

impl From<u8> for EnvelopeDirection {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Decrease,
            1 | _ => Self::Increase,
        }
    }
}

bitfield! {
  /// 4000062h, 4000068h, 4000078h - SOUND1CNT_H, SOUND2CNT_L, SOUND4CNT_L
  /// Configures duty, length and envelope for channels 1, 2 and 4
  pub struct ToneControlReg(u16);
  impl Debug;
  pub length, _: 5, 0;
  /// Ignored for channel 4 (noise)
  pub duty_pattern, _: 7, 6;
  pub envelope_step_time, _: 10, 8;
  // bitfield ignores types and `into` for single-index fields, so `11, 11` tells it to treat it
  // like a non-bool field.
  pub u8, into EnvelopeDirection, envelope_dir, _: 11, 11;
  pub envelope_init, _: 15, 12;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

bitfield! {
  /// 4000064h, 400006Ch, 4000074h - SOUND1CNT_X, SOUND2CNT_H, SOUND3CNT_X
  /// Controls frequency, length-limiting and restarting for channels 1, 2 and 3
  pub struct FrequencyReg(u16);
  impl Debug;
  pub rate, _: 10, 0;
  pub timed, _: 14;
  pub restart, _: 15;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

bitfield! {
  /// 4000070h - SOUND3CNT_L
  /// Controls enablement and RAM selection for channel 3
  pub struct WaveControlReg(u16);
  impl Debug;
  pub ram_dimension, _: 5;
  pub ram_bank_number, _: 6, 6;
  pub enable, _: 7;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

bitfield! {
  /// 4000072h - SOUND3CNT_H
  /// Controls length and volume for channel 3
  pub struct WaveLengthVolumeReg(u16);
  impl Debug;
  pub length, _: 7, 0;
  pub volume, _: 14, 13;
  pub force_volume, _: 15;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

bitfield! {
  /// 400007Ch - SOUND4CNT_H
  /// Controls frequency, length-limiting and resetting for channel 4
  pub struct NoiseFrequencyReg(u16);
  impl Debug;
  pub divide_ratio, _: 2, 0;
  pub counter_step, _: 3;
  pub shift_frequency, _: 7, 4;
  pub timed, _: 14;
  pub restart, _: 15;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

impl NoiseFrequencyReg {
    pub fn counter_width(&self) -> usize {
        if self.counter_step() {
            15
        } else {
            7
        }
    }

    pub fn shift_reg_init(&self) -> u16 {
        0x40 << (self.counter_width() - 7)
    }

    pub fn shift_xor_factor(&self) -> u16 {
        0x60 << (self.counter_width() - 7)
    }
}

pub struct LeftRight<T> {
    pub left: T,
    pub right: T,
}

bitfield! {
  /// 4000080h - SOUNDCNT_L
  /// Controls L/R volume/enablement for channels 1-4 (PSG: Programmable Sound Generator)
  pub struct PsgLeftRightReg(u16);
  impl Debug;
  pub vol_right, _: 2, 0;
  pub vol_left, _: 6, 4;
  pub enable_1_right, _: 8;
  pub enable_2_right, _: 9;
  pub enable_3_right, _: 10;
  pub enable_4_right, _: 11;
  pub enable_1_left, _: 12;
  pub enable_2_left, _: 13;
  pub enable_3_left, _: 14;
  pub enable_4_left, _: 15;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

impl PsgLeftRightReg {
    // TODO: Separate left and right audio
    pub fn psg_master_vol(&self) -> LeftRight<f32> {
        LeftRight {
            left: self.vol_left() as f32 / 7.0,
            right: self.vol_right() as f32 / 7.0,
        }
    }

    pub fn channel_enabled(&self, psg_channel_i: usize) -> LeftRight<bool> {
        let (left, right) = match psg_channel_i {
            0 => (self.enable_1_left(), self.enable_1_right()),
            1 => (self.enable_2_left(), self.enable_2_right()),
            2 => (self.enable_3_left(), self.enable_3_right()),
            3 | _ => (self.enable_4_left(), self.enable_4_right()),
        };
        LeftRight { left, right }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DmaSoundTimer {
    Timer0,
    Timer1,
}

impl From<u8> for DmaSoundTimer {
    fn from(val: u8) -> Self {
        match val {
            0 => Self::Timer0,
            1 | _ => Self::Timer1,
        }
    }
}

bitfield! {
  /// 4000082h - SOUNDCNT_H
  /// Configures DMA channel and sound mixing
  pub struct DmaControlMixReg(u16);
  impl Debug;
  pub psg_vol, _: 1, 0;
  pub dma_a_vol, _: 2;
  pub dma_b_vol, _: 3;
  pub enable_dma_a_right, _: 8;
  pub enable_dma_a_left, _: 9;
  // bitfield ignores types and `into` for single-index fields, so `10, 10` tells it to treat it
  // like a non-bool field.
  pub u8, into DmaSoundTimer, dma_a_timer, _: 10, 10;
  pub dma_a_restart, _: 11;
  pub enable_dma_b_right, _: 12;
  pub enable_dma_b_left, _: 13;
  pub u8, into DmaSoundTimer, dma_b_timer, _: 14, 14;
  pub dma_b_restart, _: 15;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

impl DmaControlMixReg {
    pub fn psg_vol_multiplier(&self) -> f32 {
        match self.psg_vol() {
            0b00 => 0.25,
            0b01 => 0.5,
            0b10 | _ => 1.0, // `3` is technically illegal
        }
    }

    pub fn dma_sound_vol_multiplier(&self, dma_channel_i: usize) -> f32 {
        let vol_bit = match dma_channel_i {
            0 => self.dma_a_vol(),
            1 | _ => self.dma_b_vol(),
        };
        if vol_bit {
            1.0
        } else {
            0.5
        }
    }

    pub fn dma_sound_enabled(&self, dma_channel_i: usize) -> LeftRight<bool> {
        let (left, right) = match dma_channel_i {
            0 => (self.enable_dma_a_left(), self.enable_dma_a_right()),
            1 | _ => (self.enable_dma_b_left(), self.enable_dma_b_right()),
        };
        LeftRight { left, right }
    }
}

bitfield! {
  /// 4000084h - SOUNDCNT_X
  /// Controls and exposes whether channels are enabled/on
  pub struct SoundOnReg(u16);
  impl Debug;
  pub psg_0_on, _: 0;
  pub psg_1_on, _: 1;
  pub psg_2_on, _: 2;
  pub psg_3_on, _: 3;
  pub master_enable, _: 7;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}

bitfield! {
  /// 4000088h - SOUNDBIAS
  /// Controls sound bias and sample rate
  pub struct SoundBiasReg(u16);
  impl Debug;
  pub bias, _: 9, 1;
  pub sample_rate, _: 15, 14;

  pub u8, lo_byte, set_lo_byte: 7, 0;
  pub u8, hi_byte, set_hi_byte: 15, 8;
}
