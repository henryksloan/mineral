use crate::PPU;

use std::convert::TryInto;

impl PPU {
    pub(super) fn update_win_masks_buf(&mut self) {
        let obj_mask = self.get_obj_win_scanline(); // OBJ mask is all false if WINOBJ disabled

        let enabled = [
            self.lcd_control_reg.enable_win0(),
            self.lcd_control_reg.enable_win1(),
            self.lcd_control_reg.enable_winobj(),
        ];
        if enabled.iter().all(|x| !x) {
            self.win_mask_bufs = [[true; 6]; 240];
            return;
        }

        let y = self.scan_line as usize;
        for x in 0..240 {
            let win_n = if enabled[0] && self.pixel_in_window(0, x, y) {
                0
            } else if enabled[1] && self.pixel_in_window(1, x, y) {
                1
            } else if obj_mask[x] {
                2
            } else {
                3
            };
            self.win_mask_bufs[x] = self.get_win_layers_enabled(win_n);
        }
    }

    fn pixel_in_window(&self, win_n: usize, x: usize, y: usize) -> bool {
        let coords = if win_n == 0 {
            &self.win0_coords
        } else {
            &self.win1_coords
        };

        // TODO: Offscreen and reverse-order values should possibly work differently
        let x_range = {
            let lo = coords.0.coord_lo() as usize;
            let mut hi = coords.0.coord_hi() as usize;
            if hi > 240 || lo > hi {
                hi = 240;
            }
            lo..hi
        };
        let y_range = {
            let lo = coords.1.coord_lo() as usize;
            let mut hi = coords.1.coord_hi() as usize;
            if hi > 160 || lo > hi {
                hi = 160;
            }
            lo..hi
        };

        x_range.contains(&x) && y_range.contains(&y)
    }

    fn get_win_layers_enabled(&self, win_n: usize) -> [bool; 6] {
        let mask = match win_n {
            0 => self.win_inside.lo_byte(),      // WIN0
            1 => self.win_inside.hi_byte(),      // WIN1
            2 => self.win_outside.hi_byte(),     // WINOBJ
            3 | _ => self.win_outside.lo_byte(), // WINOUT
        };
        (0..6)
            .map(|i| (mask >> i) & 1 == 1)
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }
}
