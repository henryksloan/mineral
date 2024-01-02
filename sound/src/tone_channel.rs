use crate::registers::*;

pub struct ToneChannel {
    // Channel 2 doesn't support tone sweep, so this register is unmodifiable via IO for that channel.
    pub sweep_reg: ToneSweepReg,
    pub control_reg: ToneControlReg,
    frequency_reg: FrequencyReg,

    counter: u32,
    period: u32,
    curr_vol: u16,
}

impl ToneChannel {
    pub fn new() -> Self {
        Self {
            sweep_reg: ToneSweepReg(0),
            control_reg: ToneControlReg(0),
            frequency_reg: FrequencyReg(0),

            counter: 0,
            period: 0,
            curr_vol: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.counter > 0 {
            self.counter -= 1;
        } else {
            self.counter = self.period;
        }
    }

    pub fn sample(&self) -> f32 {
        // TODO: DO NOT SUBMIT: Duty cycle, etc.
        let vol = (self.curr_vol as f32) / 15.0;
        if self.counter < (self.period / 2) {
            vol
        } else {
            -vol
        }
    }

    pub fn set_frequency_reg_lo(&mut self, data: u8) {
        self.frequency_reg.set_lo_byte(data);
        self.update_period();
    }

    pub fn set_frequency_reg_hi(&mut self, data: u8) {
        self.frequency_reg.set_hi_byte(data);
        self.update_period();
        if self.frequency_reg.restart() {
            self.restart();
        }
    }

    fn update_period(&mut self) {
        self.period = 16_777_216 / (131072 / (2048 - self.frequency_reg.rate() as u32));
    }

    fn restart(&mut self) {
        self.counter = self.period;
        self.curr_vol = self.control_reg.envelope_init();
    }
}
