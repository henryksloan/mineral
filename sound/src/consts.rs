pub const MASTER_CLOCK_HZ: u32 = 16_777_216;

// The length counter is in units of 1/256 seconds, so this represents the number of clock ticks
// per length counter decrement.
pub const LENGTH_UNIT_PERIOD: u32 = MASTER_CLOCK_HZ / 256;
// The envelope counter is in units of 1/64 seconds.
pub const ENVELOPE_UNIT_PERIOD: u32 = MASTER_CLOCK_HZ / 64;
// The sweep counter is in units of 1/64 seconds.
pub const SWEEP_UNIT_PERIOD: u32 = MASTER_CLOCK_HZ / 128;
