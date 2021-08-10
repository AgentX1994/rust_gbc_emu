use std::{
    fs::File,
    io::{BufWriter, Write},
};

use super::cpu::InterruptRequest;

#[derive(Debug)]
pub struct Joypad {
    input: u8, // (action << 4) | directions, a 0 means pressed
    selected: u8
}

impl Default for Joypad {
    fn default() -> Self {
        Self {input: 0xff, 
            selected: 0 }
    }
}

impl Joypad {
    pub fn write_u8(&mut self, byte: u8) {
        let byte = byte & 0x30; // mask out only the select bits;
        self.selected = byte;
    }

    pub fn read_u8(&self) -> u8 {
        if self.selected & (1 << 4) != 0 {
            0xd | self.selected | (self.input & 0xf)
        } else if self.selected & (1 << 5) != 0 {
            0xd | self.selected | ((self.input >> 4) & 0xf)
        } else {
            0xd | self.selected
        }
    }
}

#[derive(Debug)]
pub struct SerialComms {
    pub io_register: u8,
    pub control: u8,
    io_register_on_control_byte_write: u8,
    ticks: u64,
    bits_written: u8,
    out_byte: u8,
    out_file: File,
}

impl SerialComms {
    pub fn tick(&mut self, _cycles: u64) -> bool {
        if self.control & 0x80 == 0 {
            self.ticks = 0;
            return false;
        }

        self.out_file.write_all(&[self.io_register]).expect("can't write to file");
        self.out_file.flush().expect("Could not flush");
        self.control = self.control & 0x7f;
        true
        
        // const CYCLES_PER_BYTE: u64 = 4194304 / 8192; // CPU speed (4194304 HZ) divided by internal clock (8192 HZ)
        // self.ticks += cycles;
        // let mut interrupt_required = false;
        // while self.ticks > CYCLES_PER_BYTE {
        //     self.ticks -= CYCLES_PER_BYTE;
        //     self.out_byte = (self.out_byte << 1) | (self.io_register >> 7);
        //     self.io_register <<= 1;
        //     self.bits_written += 1;
        //     if self.bits_written == 8 {
        //         if self.out_byte != self.io_register_on_control_byte_write {
        //             println!("WARNING: Serial IO register was changed during serial output! {} {:#02x} became {:#02x}", self.io_register_on_control_byte_write as char, self.io_register_on_control_byte_write, self.out_byte);
        //         }
        //         self.out_file
        //             .write_all(&[self.out_byte])
        //             .expect("can't write to file!");
        //         self.out_file.flush().expect("Could not flush");
        //         self.out_byte = 0;
        //         self.bits_written = 0;
        //         self.control = self.control & 0x7f;

        //         interrupt_required = true;
        //     } else {
        //         interrupt_required = false;
        //     }
        // }
        // interrupt_required
    }
}

impl Default for SerialComms {
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

#[derive(Debug, Default)]
pub struct Timer {
    pub divider: u8,
    pub timer_counter: u8,
    pub timer_reset_value: u8,
    pub control: u8,
}

#[derive(Debug, Default)]
pub struct SoundChannel {
    pub sweep_control: u8,
    pub sound_length_duty: u8,
    pub volume_envelope: u8,
    pub frequency_low: u8,
    pub frequency_high: u8,
}

#[derive(Debug, Default)]
pub struct SoundDigitalChannel {
    pub is_on: bool,
    pub length: u8,
    pub volume: u8,
    pub frequency_low: u8,
    pub frequency_high: u8,
    pub wave_ram: [u8; 16],
}

#[derive(Debug, Default)]
pub struct SoundNoiseChannel {
    pub length: u8,
    pub volume: u8,
    pub polynomial_counter: u8,
    pub control: u8,
}

#[derive(Debug, Default)]
pub struct Sound {
    pub channel_1: SoundChannel,
    pub channel_2: SoundChannel,
    pub digital_channel: SoundDigitalChannel,
    pub noise_channel: SoundNoiseChannel,
    pub channel_control: u8,
    pub sound_output_control: u8,
    pub sound_on_off_control: u8,
}

#[derive(Debug)]
pub struct Lcd {
    pub control: u8,
    pub status: u8,
    pub scroll_y: u8,
    pub scroll_x: u8,
    pub ly: u8,
    pub ly_compare: u8,
    pub dma_start_high_byte: u8,
    pub background_palette: u8,
    pub object_palette_0: u8,
    pub object_pallete_1: u8,
    pub window_y: u8,
    pub window_x: u8,
}

impl Default for Lcd {
    fn default() -> Self {
        Self {
            control: 0,
            status: 0,
            scroll_y: 0,
            scroll_x: 0,
            ly: 144,
            ly_compare: 0,
            dma_start_high_byte: 0,
            background_palette: 0,
            object_palette_0: 0,
            object_pallete_1: 0,
            window_y: 0,
            window_x: 0,
        }
    }
}

#[derive(Debug)]
pub struct Mmio {
    pub joypad: Joypad,
    pub serial: SerialComms,
    pub timer_control: Timer,
    pub sound: Sound,
    pub lcd: Lcd,
    pub vram_select: u8,
    pub disable_boot_rom: bool,
    pub vram_dma: [u8; 4],
    pub color_palettes: [u8; 2],
    pub wram_bank_select: u8,
}

impl Default for Mmio {
    fn default() -> Self {
        Self {
            joypad: Joypad::default(),
            serial: SerialComms::default(),
            timer_control: Timer::default(),
            sound: Sound::default(),
            lcd: Lcd::default(),
            vram_select: 0,
            disable_boot_rom: false,
            vram_dma: [0; 4],
            color_palettes: [0; 2],
            wram_bank_select: 0,
        }
    }
}

impl Mmio {
    pub fn read_u8(&self, address: u16) -> u8 {
        match address {
            0x00 => self.joypad.read_u8(),
            0x01 => self.serial.io_register,
            0x02 => self.serial.control,
            0x40 => self.lcd.control,
            0x41 => self.lcd.status,
            0x42 => self.lcd.scroll_y,
            0x43 => self.lcd.scroll_x,
            0x44 => self.lcd.ly,
            0x45 => self.lcd.ly_compare,
            0x46 => self.lcd.dma_start_high_byte,
            0x47 => self.lcd.background_palette,
            0x48 => self.lcd.object_palette_0,
            0x49 => self.lcd.object_pallete_1,
            0x4a => self.lcd.window_y,
            0x4b => self.lcd.window_x,
            _ => 0, // TODO
        }
    }

    pub fn write_u8(&mut self, address: u16, byte: u8) {
        match address {
            0x00 => self.joypad.write_u8(byte),
            0x01 => self.serial.io_register = byte,
            0x02 => {
                self.serial.control = byte;
                if byte & 0x80 != 0 {
                    self.serial.io_register_on_control_byte_write = self.serial.io_register;
                }
            }
            0x40 => self.lcd.control = byte,
            0x41 => self.lcd.status = byte,
            0x42 => self.lcd.scroll_y = byte,
            0x43 => self.lcd.scroll_x = byte,
            0x44 => self.lcd.ly = byte,
            0x45 => self.lcd.ly_compare = byte,
            0x46 => self.lcd.dma_start_high_byte = byte,
            0x47 => self.lcd.background_palette = byte,
            0x48 => self.lcd.object_palette_0 = byte,
            0x49 => self.lcd.object_pallete_1 = byte,
            0x4a => self.lcd.window_y = byte,
            0x4b => self.lcd.window_x = byte,
            _ => (), // TODO
        }
    }

    pub fn tick(&mut self, cycles: u64) -> InterruptRequest {
        let mut interrupts = InterruptRequest::default();

        interrupts.serial = self.serial.tick(cycles);

        interrupts
    }
}
