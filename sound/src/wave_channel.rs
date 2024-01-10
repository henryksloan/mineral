use crate::consts::*;
use crate::registers::*;

pub struct WaveChannel {
    pub control_reg: WaveControlReg,
    pub length_volume_reg: WaveLengthVolumeReg,
    frequency_reg: FrequencyReg,

    // From MSB to LSB, the 128-bit integers store WAVE_RAM0, WAVE_RAM1, WAVE_RAM2, WAVE_RAM3.
    // The most significant 4 bits are output at any given time, and the pattern is progressed by
    // shifting the playing region (the non-selected bank in 32-digit mode, or the whole RAM in
    // 64-digit mode) 4 bits to the left. This accords with the real hardware, where the pattern
    // RAM is made up of 128-bit shift registers played in this same order.
    pattern_ram: [u128; 2],
    playing_octet: usize,
    playing_other_bank: bool,
    counter: u32,
    length_counter: u32,
    length_divider: u32,
}

impl WaveChannel {
    pub fn new() -> Self {
        Self {
            control_reg: WaveControlReg(0),
            length_volume_reg: WaveLengthVolumeReg(0),
            frequency_reg: FrequencyReg(0),

            pattern_ram: [0; 2],
            playing_octet: 0,
            playing_other_bank: false,
            counter: 0,
            length_counter: 0,
            length_divider: 0,
        }
    }

    pub fn sample(&self) -> f32 {
        if !self.control_reg.enable() || (self.frequency_reg.timed() && self.length_counter == 0) {
            return 0.0;
        }
        // TODO: DO NOT SUBMIT: How should this behave for e.g. all zeroes?
        self.volume_multiplier()
            * (((self.pattern_ram[self.playing_bank_i()] >> 124) as f32 / 15.0) - 0.5)
    }

    fn volume_multiplier(&self) -> f32 {
        if self.length_volume_reg.force_volume() {
            0.75
        } else {
            match self.length_volume_reg.volume() {
                0 => 0.0,
                1 => 1.0,
                2 => 0.5,
                3 | _ => 0.25,
            }
        }
    }

    fn playing_bank_i(&self) -> usize {
        if self.control_reg.ram_dimension() && !self.playing_other_bank {
            self.control_reg.ram_bank_number() as usize
        } else {
            1 - self.control_reg.ram_bank_number() as usize
        }
    }

    pub fn tick(&mut self) {
        if self.control_reg.enable() {
            self.tick_wave();
        }
        self.tick_length();
    }

    // `octet_i` represents the i'th pattern RAM register (4000090h, 4000091h, 4000092h, etc.),
    // although `pattern_ram` stores them in the opposite order.
    pub fn read_pattern_octet(&mut self, octet_i: usize) -> u8 {
        let bank_i = self.control_reg.ram_bank_number() as usize;
        let offset = 8 * (7 - octet_i);
        ((self.pattern_ram[bank_i] >> offset) & 0xFF) as u8
    }

    pub fn write_pattern_octet(&mut self, octet_i: usize, data: u8) {
        let bank_i = self.control_reg.ram_bank_number() as usize;
        let offset = 8 * (7 - octet_i);
        self.pattern_ram[bank_i] &= !(0xFF << offset);
        self.pattern_ram[bank_i] |= (data as u128) << offset;
    }

    fn tick_wave(&mut self) {
        if self.counter > 0 {
            self.counter -= 1;
        } else {
            self.counter = self.period();
            self.progress_playback();
        }
    }

    fn progress_playback(&mut self) {
        let playing_bank_i = self.playing_bank_i();
        self.pattern_ram[playing_bank_i] = self.pattern_ram[playing_bank_i].rotate_left(4);
        if self.playing_octet < 7 {
            self.playing_octet += 1;
        } else {
            self.playing_octet = 0;
            if self.control_reg.ram_dimension() {
                self.playing_other_bank = !self.playing_other_bank;
            }
        }
    }

    fn tick_length(&mut self) {
        if self.length_divider > 0 {
            self.length_divider -= 1;
        } else {
            self.length_divider = LENGTH_UNIT_PERIOD;
            if self.length_counter > 0 {
                self.length_counter -= 1;
            }
        }
    }

    pub fn set_frequency_reg_lo(&mut self, data: u8) {
        self.frequency_reg.set_lo_byte(data);
    }

    pub fn set_frequency_reg_hi(&mut self, data: u8) {
        self.frequency_reg.set_hi_byte(data);
        if self.frequency_reg.restart() {
            self.restart();
        }
    }

    fn restart(&mut self) {
        self.counter = self.period();
        self.length_counter = 256 - self.length_volume_reg.length() as u32;
        self.length_divider = LENGTH_UNIT_PERIOD;
        self.playing_octet = 0;
        self.playing_other_bank = false;
    }

    fn period(&self) -> u32 {
        MASTER_CLOCK_HZ / (2_097_152 / (2048 - self.frequency_reg.rate() as u32))
    }
}
