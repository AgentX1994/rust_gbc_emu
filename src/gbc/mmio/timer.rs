// TODO support other clock speeds
const DIV_TICK_RATE: u64 = 16384;
const CPU_FREQ: u64 = 4194304;
const CYCLES_PER_DIV_TICK: u64 = CPU_FREQ / DIV_TICK_RATE;

#[derive(Debug)]
pub struct Timer {
    divider: u8,
    timer_counter: u8,
    timer_reset_value: u8,
    control: u8,
    div_cycles_counter: u64,
    tma_cycles_counter: u64,
}

impl Default for Timer {
    fn default() -> Self {
        Self {
            divider: 0x18,
            timer_counter: 0,
            timer_reset_value: 0,
            control: 0,
            div_cycles_counter: 0,
            tma_cycles_counter: 0,
        }
    }
}

impl Timer {
    pub fn read_u8(&self, offset: u16) -> u8 {
        match offset {
            0 => self.divider,
            1 => self.timer_counter,
            2 => self.timer_reset_value,
            3 => self.control,
            _ => unreachable!(),
        }
    }

    pub fn write_u8(&mut self, offset: u16, byte: u8) {
        match offset {
            0 => self.divider = 0, // writes reset the divider to zero
            1 => self.timer_counter = byte,
            2 => self.timer_reset_value = byte,
            3 => self.control = byte & 0x7, // 3 bit register
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self, cycles: u64) -> bool {
        // check if div should be ticked
        self.div_cycles_counter += cycles;
        if self.div_cycles_counter > CYCLES_PER_DIV_TICK {
            self.divider = self.divider.wrapping_add(1);
            self.div_cycles_counter -= CYCLES_PER_DIV_TICK;
        }

        // check if TIMA should be ticked
        if self.control & 0x4 != 0 {
            self.tma_cycles_counter += cycles;
            let cycles_per_tma_tick = match self.control & 0x3 {
                0 => CPU_FREQ / 4096,
                1 => CPU_FREQ / 262144,
                2 => CPU_FREQ / 65536,
                3 => CPU_FREQ / 16384,
                _ => unreachable!(),
            };
            if self.tma_cycles_counter > cycles_per_tma_tick {
                self.tma_cycles_counter -= cycles_per_tma_tick;
                let res = self.timer_counter.checked_add(1);
                match res {
                    Some(v) => {
                        self.timer_counter = v;
                        false
                    }
                    None => {
                        self.timer_counter = self.timer_reset_value;
                        true
                    }
                }
            } else {
                false
            }
        } else {
            false
        }
    }
}
