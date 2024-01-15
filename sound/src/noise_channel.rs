use crate::consts::*;
use crate::registers::*;

pub struct NoiseChannel {
    control_reg: ToneControlReg,
    frequency_reg: NoiseFrequencyReg,

    counter: u32,
    output_high: bool,
    polynomial_shift_reg: u16,
    curr_vol: u16,
    length_counter: u32,
    length_divider: u32,
    envelope_counter: u32,
    envelope_divider: u32,
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            control_reg: ToneControlReg(0),
            frequency_reg: NoiseFrequencyReg(0),

            counter: 0,
            output_high: false,
            polynomial_shift_reg: 0,
            curr_vol: 0,
            length_counter: 0,
            length_divider: 0,
            envelope_counter: 0,
            envelope_divider: 0,
        }
    }

    // TODO: Deduplicate a lot of this from ToneChannel
    pub fn tick(&mut self) {
        self.tick_wave();
        self.tick_length();
        self.tick_envelope();
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
        self.output_high = (self.polynomial_shift_reg & 1) == 1;
        self.polynomial_shift_reg >>= 1;
        if self.output_high {
            self.polynomial_shift_reg ^= self.frequency_reg.shift_xor_factor();
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

    fn tick_envelope(&mut self) {
        if self.envelope_divider > 0 {
            self.envelope_divider -= 1;
        } else {
            self.envelope_divider = ENVELOPE_UNIT_PERIOD;
            if self.envelope_counter > 0 {
                self.envelope_counter -= 1;
            } else {
                self.envelope_counter = self.control_reg.envelope_step_time() as u32;
                if self.envelope_counter == 0 {
                    return;
                }
                self.curr_vol = match self.control_reg.envelope_dir() {
                    EnvelopeDirection::Decrease => self.curr_vol.saturating_sub(1),
                    EnvelopeDirection::Increase => std::cmp::min(self.curr_vol + 1, 15),
                }
            }
        }
    }

    fn restart(&mut self) {
        self.counter = self.period();
        self.output_high = false;
        self.polynomial_shift_reg = self.frequency_reg.shift_reg_init();
        self.curr_vol = self.control_reg.envelope_init();
        self.length_counter = 64 - self.control_reg.length() as u32;
        self.length_divider = LENGTH_UNIT_PERIOD;
        self.envelope_counter = self.control_reg.envelope_step_time() as u32;
        self.envelope_divider = ENVELOPE_UNIT_PERIOD;
    }

    pub fn sample(&self) -> f32 {
        if self.frequency_reg.timed() && self.length_counter == 0 {
            return 0.0;
        }
        let vol = (self.curr_vol as f32) / 15.0;
        if self.output_high {
            vol
        } else {
            -vol
        }
    }

    pub fn set_control_reg_lo(&mut self, data: u8) {
        self.control_reg.set_lo_byte(data);
    }

    // TODO: Deduplicate some of this register read/write logic from ToneChannel
    // Maybe by encapsulating the register and its computed attributes/callbacks in a struct?
    pub fn set_control_reg_hi(&mut self, data: u8) {
        self.control_reg.set_hi_byte(data);
        // Writing zeroes to bits 3-7 of this half of the control register immediately turns off
        // the channel.
        if self.control_reg.envelope_dir() == EnvelopeDirection::Decrease
            && self.control_reg.envelope_init() == 0
        {
            self.curr_vol = 0;
        }
    }

    pub fn control_reg_lo(&self) -> u8 {
        self.control_reg.lo_byte()
    }

    pub fn control_reg_hi(&self) -> u8 {
        self.control_reg.hi_byte()
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

    pub fn frequency_reg_lo(&self) -> u8 {
        self.frequency_reg.lo_byte()
    }

    pub fn frequency_reg_hi(&self) -> u8 {
        self.frequency_reg.hi_byte()
    }

    fn period(&self) -> u32 {
        const MULTIPLIER_HZ: u32 = 524_288;
        let base_freq = match self.frequency_reg.divide_ratio() {
            0 => MULTIPLIER_HZ * 2,
            1 => MULTIPLIER_HZ,
            2 | _ => MULTIPLIER_HZ / 2,
        };
        let factor = 1u32 << (self.frequency_reg.shift_frequency() + 1);
        MASTER_CLOCK_HZ / (base_freq / factor)
    }
}
