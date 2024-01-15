use std::collections::VecDeque;

pub struct DmaSoundChannel {
    fifo: VecDeque<u32>,
    fifo_octet_i: usize,
}

impl DmaSoundChannel {
    pub fn new() -> Self {
        Self {
            fifo: VecDeque::with_capacity(8),
            fifo_octet_i: 0,
        }
    }

    // Handles a write to one of the FIFO data registers. A write to the 0th octet pushes a new
    // entry into the FIFO, while writes to the other offset simply overwrite octets in the most
    // recent entry. I'm not sure if this is realistic, but I also don't think out-of-order writes
    // to the 8-bit FIFO registers are well-defined.
    pub fn write_fifo_octet(&mut self, octet_i: usize, data: u8) {
        if octet_i == 0 {
            self.fifo.push_back(0);
        }
        let fifo_entry = self.fifo.back_mut().unwrap();
        let octet_offset = octet_i * 8;
        let octet_mask = 0xFFu32 << octet_offset;
        *fifo_entry &= !octet_mask;
        *fifo_entry |= (data as u32) << octet_offset;
    }

    /// Returns true if a DMA is to be requested
    pub fn tick_fifo(&mut self) -> bool {
        if self.fifo_octet_i < 3 {
            self.fifo_octet_i += 1;
            false
        } else {
            self.fifo.pop_front();
            self.fifo_octet_i = 0;
            self.fifo.len() <= 4
        }
    }

    pub fn restart(&mut self) {
        self.fifo.clear();
        self.fifo_octet_i = 0;
    }

    pub fn sample(&self) -> f32 {
        if let Some(val) = self.fifo.front() {
            ((val >> (self.fifo_octet_i * 8)) & 0xFF) as i8 as f32 / 128.
        } else {
            0.0
        }
    }
}
