use std::{fs::File, io::Write};

const CYCLES_PER_BYTE: u64 = 4_194_304 / 8192; // CPU speed (4194304 HZ) divided by internal clock (8192 HZ)

#[derive(Debug)]
pub struct Comms {
    pub io_register: u8,
    pub control: u8,
    pub io_register_on_control_byte_write: u8,
    ticks: u64,
    bits_written: u8,
    out_byte: u8,
    out_file: File,
}

impl Comms {
    #[must_use]
    pub fn read_u8(&self, offset: u16) -> u8 {
        match offset {
            0 => self.io_register,
            1 => self.control,
            _ => unreachable!(),
        }
    }

    pub fn write_u8(&mut self, offset: u16, byte: u8) {
        match offset {
            0 => self.io_register = byte,
            1 => {
                self.control = byte;
                if (byte & 0x80) != 0 {
                    self.io_register_on_control_byte_write = self.io_register;
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self, cycles: u64) -> bool {
        if self.control & 0x80 == 0 {
            self.ticks = 0;
            return false;
        }

        self.ticks += cycles;
        let mut interrupt_required = false;
        while self.ticks > CYCLES_PER_BYTE {
            self.ticks -= CYCLES_PER_BYTE;
            self.out_byte = (self.out_byte << 1) | (self.io_register >> 7);
            self.io_register <<= 1;

            // Per https://gbdev.io/pandocs/Interrupt_Sources.html#int-58---serial-interrupt
            // 0xff should be received if there is no other game boy present
            self.io_register |= 1;

            self.bits_written += 1;
            if self.bits_written == 8 {
                if self.out_byte != self.io_register_on_control_byte_write {
                    println!("WARNING: Serial IO register was changed during serial output! {} {:#02x} became {:#02x}", self.io_register_on_control_byte_write as char, self.io_register_on_control_byte_write, self.out_byte);
                }
                self.out_file
                    .write_all(&[self.out_byte])
                    .expect("can't write to file!");
                self.out_file.flush().expect("Could not flush");
                self.out_byte = 0;
                self.bits_written = 0;
                self.control &= 0x7f;

                interrupt_required = true;
            } else {
                interrupt_required = false;
            }
        }
        interrupt_required
    }
}

impl Default for Comms {
    fn default() -> Self {
        Self {
            io_register: 0,
            control: 0,
            io_register_on_control_byte_write: 0,
            ticks: 0,
            bits_written: 0,
            out_byte: 0,
            out_file: //BufWriter::new(
                File::create("serial_out.dat").expect("Can't open serial_out.dat!"),
            //),
        }
    }
}
