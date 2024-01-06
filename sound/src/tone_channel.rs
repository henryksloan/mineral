use crate::registers::*;

// The length counter is in units of 1/256 seconds, so this represents the number of clock ticks
// per length counter decrement.
const LENGTH_UNIT_PERIOD: u32 = 16777216 / 256;

pub struct ToneChannel {
    // Channel 2 doesn't support tone sweep, so this register is unmodifiable via IO for that channel.
    pub sweep_reg: ToneSweepReg,
    pub control_reg: ToneControlReg,
    frequency_reg: FrequencyReg,

    counter: u32,
    curr_vol: u16,
    length_counter: u32,
    length_divider: u32,
}

impl ToneChannel {
    pub fn new() -> Self {
        Self {
            sweep_reg: ToneSweepReg(0),
            control_reg: ToneControlReg(0),
            frequency_reg: FrequencyReg(0),

            counter: 0,
            curr_vol: 0,
            length_counter: 0,
            length_divider: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.counter > 0 {
            self.counter -= 1;
        } else {
            self.counter = self.period();
        }

        if self.length_divider > 0 {
            self.length_divider -= 1;
        } else {
            self.length_divider = LENGTH_UNIT_PERIOD;
            if self.length_counter > 0 {
                self.length_counter -= 1;
            }
        }
    }

    pub fn restart(&mut self) {
        self.counter = self.period();
        self.curr_vol = self.control_reg.envelope_init();
        self.length_counter = 64 - self.control_reg.length() as u32;
        self.length_divider = LENGTH_UNIT_PERIOD;
    }

    pub fn sample(&self) -> f32 {
        // TODO: DO NOT SUBMIT: Envelope, frequency sweep
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

    pub fn set_frequency_reg_lo(&mut self, data: u8) {
        self.frequency_reg.set_lo_byte(data);
    }

    pub fn set_frequency_reg_hi(&mut self, data: u8) {
        self.frequency_reg.set_hi_byte(data);
        if self.frequency_reg.restart() {
            self.restart();
        }
    }

    fn period(&self) -> u32 {
        16_777_216 / (131072 / (2048 - self.frequency_reg.rate() as u32))
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
