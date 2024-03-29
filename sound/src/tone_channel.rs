use crate::consts::*;
use crate::registers::*;

pub struct ToneChannel {
    // Channel 2 doesn't support tone sweep, so this register is unmodifiable via IO for that channel.
    pub sweep_reg: ToneSweepReg,
    control_reg: ToneControlReg,
    frequency_reg: FrequencyReg,

    curr_rate: u16,
    counter: u32,
    curr_vol: u16,
    length_counter: u32,
    length_divider: u32,
    envelope_counter: u32,
    envelope_divider: u32,
    sweep_counter: u32,
    sweep_divider: u32,
}

impl ToneChannel {
    pub fn new() -> Self {
        Self {
            sweep_reg: ToneSweepReg(0),
            control_reg: ToneControlReg(0),
            frequency_reg: FrequencyReg(0),

            curr_rate: 0,
            counter: 0,
            curr_vol: 0,
            length_counter: 0,
            length_divider: 0,
            envelope_counter: 0,
            envelope_divider: 0,
            sweep_counter: 0,
            sweep_divider: 0,
        }
    }

    pub fn tick(&mut self) {
        self.tick_wave();
        self.tick_length();
        self.tick_envelope();
        self.tick_sweep();
    }

    fn tick_wave(&mut self) {
        if self.counter > 0 {
            self.counter -= 1;
        } else {
            self.counter = self.period();
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

    fn tick_sweep(&mut self) {
        if self.sweep_divider > 0 {
            self.sweep_divider -= 1;
        } else {
            self.sweep_divider = SWEEP_UNIT_PERIOD;
            if self.sweep_counter > 0 {
                self.sweep_counter -= 1;
            } else {
                self.sweep_counter = self.sweep_reg.sweep_time() as u32;
                if self.sweep_counter != 0 {
                    let delta_rate = self.curr_rate / (1 << self.sweep_reg.sweep_shift_n());
                    self.curr_rate = match self.sweep_reg.sweep_dir() {
                        SweepDirection::Decrease => self.curr_rate.saturating_sub(delta_rate),
                        SweepDirection::Increase => {
                            std::cmp::min(self.curr_rate + delta_rate, 2047)
                        }
                    }
                }
            }
        }
    }

    fn restart(&mut self) {
        self.curr_rate = self.frequency_reg.rate();
        self.counter = self.period();
        self.curr_vol = self.control_reg.envelope_init();
        self.length_counter = 64 - self.control_reg.length() as u32;
        self.length_divider = LENGTH_UNIT_PERIOD;
        self.envelope_counter = self.control_reg.envelope_step_time() as u32;
        self.envelope_divider = ENVELOPE_UNIT_PERIOD;
        self.sweep_counter = self.sweep_reg.sweep_time() as u32;
        self.sweep_divider = SWEEP_UNIT_PERIOD;
    }

    pub fn sample(&self) -> f32 {
        if self.frequency_reg.timed() && self.length_counter == 0 {
            return 0.0;
        }
        let vol = (self.curr_vol as f32) / 15.0;
        if self.counter < self.duty_high_width() {
            vol
        } else {
            -vol
        }
    }

    pub fn set_control_reg_lo(&mut self, data: u8) {
        self.control_reg.set_lo_byte(data);
    }

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
        MASTER_CLOCK_HZ / (131_072 / (2048 - self.curr_rate as u32))
    }

    fn duty_high_width(&self) -> u32 {
        let period = self.period();
        match self.control_reg.duty_pattern() {
            0 => period / 8,
            1 => period / 4,
            2 => period / 2,
            3 | _ => (3 * period) / 4,
        }
    }
}
