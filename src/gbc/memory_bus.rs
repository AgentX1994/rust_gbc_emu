use std::convert::TryInto;
use std::sync::{Arc, Mutex};

use super::cartridge::Cartridge;
use super::mmio::{apu::Sound, joypad::Joypad, lcd::Lcd, serial::SerialComms, timer::Timer};
use super::ppu::PictureProcessingUnit;

enum MemoryRegion {
    CartridgeBank0(u16),
    CartridgeBankSelectable(u16),
    VideoRam(u16),
    ExternalRam(u16),
    WorkRam(u16),
    ObjectAttributeMemory(u16),
    Unused,
    Joypad,
    Serial(u16),
    Timer(u16),
    InterruptFlags,
    Sound(u16),
    WaveformRam(u16),
    Lcd(u16),
    Key1Flag,
    HighRam(u16),
    InterruptEnable,
}

impl From<u16> for MemoryRegion {
    fn from(address: u16) -> Self {
        match address {
            0x0000..=0x3fff => MemoryRegion::CartridgeBank0(address),
            0x4000..=0x7fff => MemoryRegion::CartridgeBankSelectable(address - 0x4000),
            0x8000..=0x9fff => MemoryRegion::VideoRam(address - 0x8000),
            0xa000..=0xbfff => MemoryRegion::ExternalRam(address - 0xa000),
            0xc000..=0xdfff => MemoryRegion::WorkRam(address - 0xc000),
            0xe000..=0xfdff => MemoryRegion::WorkRam(address - 0xe000),
            0xfe00..=0xfe9f => MemoryRegion::ObjectAttributeMemory(address - 0xfe00),
            0xfea0..=0xfeff => MemoryRegion::Unused,
            0xff00 => MemoryRegion::Joypad,
            0xff01..=0xff02 => MemoryRegion::Serial(address - 0xff01),
            0xff04..=0xff07 => MemoryRegion::Timer(address - 0xff04),
            0xff0f => MemoryRegion::InterruptFlags,
            0xff10..=0xff26 => MemoryRegion::Sound(address - 0xff10),
            0xff30..=0xff3f => MemoryRegion::WaveformRam(address - 0xff30),
            0xff40..=0xff4b => MemoryRegion::Lcd(address - 0xff40),
            0xff4d => MemoryRegion::Key1Flag,
            0xff80..=0xfffe => MemoryRegion::HighRam(address - 0xff80),
            0xffff => MemoryRegion::InterruptEnable,
            _ => MemoryRegion::Unused,
        }
    }
}

#[derive(Debug)]
pub struct MemoryBus {
    pub cartridge: Arc<Mutex<Cartridge>>,
    pub ram: Arc<Mutex<[u8; 8192]>>,
    pub ppu: Arc<Mutex<PictureProcessingUnit>>,
    pub joypad: Arc<Mutex<Joypad>>,
    pub serial: Arc<Mutex<SerialComms>>,
    pub timer_control: Arc<Mutex<Timer>>,
    pub sound: Arc<Mutex<Sound>>,
    pub lcd: Arc<Mutex<Lcd>>,
    pub vram_select: Arc<Mutex<u8>>,
    pub disable_boot_rom: Arc<Mutex<bool>>,
    pub vram_dma: Arc<Mutex<[u8; 4]>>,
    pub color_palettes: Arc<Mutex<[u8; 2]>>,
    pub wram_bank_select: Arc<Mutex<u8>>,
    pub interrupt_flags: Arc<Mutex<u8>>,
    pub high_ram: Arc<Mutex<[u8; 127]>>,
    pub interrupt_enable: Arc<Mutex<u8>>,
}

impl MemoryBus {
    pub fn new(
        cartridge: Arc<Mutex<Cartridge>>,
        ram: Arc<Mutex<[u8; 8192]>>,
        ppu: Arc<Mutex<PictureProcessingUnit>>,
        joypad: Arc<Mutex<Joypad>>,
        serial: Arc<Mutex<SerialComms>>,
        timer_control: Arc<Mutex<Timer>>,
        sound: Arc<Mutex<Sound>>,
        lcd: Arc<Mutex<Lcd>>,
        vram_select: Arc<Mutex<u8>>,
        disable_boot_rom: Arc<Mutex<bool>>,
        vram_dma: Arc<Mutex<[u8; 4]>>,
        color_palettes: Arc<Mutex<[u8; 2]>>,
        wram_bank_select: Arc<Mutex<u8>>,
        interrupt_flags: Arc<Mutex<u8>>,
        high_ram: Arc<Mutex<[u8; 127]>>,
        interrupt_enable: Arc<Mutex<u8>>,
    ) -> Self {
        MemoryBus {
            cartridge,
            ram,
            ppu,
            joypad,
            serial,
            timer_control,
            sound,
            lcd,
            vram_select,
            disable_boot_rom,
            vram_dma,
            color_palettes,
            wram_bank_select,
            interrupt_flags,
            high_ram,
            interrupt_enable: interrupt_enable,
        }
    }

    pub fn read_u8(&self, address: u16) -> u8 {
        let region = MemoryRegion::from(address);
        match region {
            MemoryRegion::CartridgeBank0(offset) => self.cartridge.lock().unwrap().read_rom_bank_0(offset),
            MemoryRegion::CartridgeBankSelectable(offset) => {
                self.cartridge.lock().unwrap().read_rom_selected_bank(offset)
            }
            MemoryRegion::VideoRam(offset) => self.ppu.lock().unwrap().read_video_ram(offset),
            MemoryRegion::ExternalRam(offset) => {
                self.cartridge.lock().unwrap().read_from_external_ram(offset)
            }
            MemoryRegion::WorkRam(offset) => self.ram.lock().unwrap()[offset as usize],
            MemoryRegion::ObjectAttributeMemory(offset) => {
                self.ppu.lock().unwrap().read_object_attribute_memory(offset)
            }
            MemoryRegion::Unused => {
                // Use Color Game Boy Revision E behavior I guess?
                let second_nibble = ((address >> 4) & 0xf) as u8;
                (second_nibble << 4) | second_nibble
            }
            MemoryRegion::Joypad => self.joypad.lock().unwrap().read_u8(),
            MemoryRegion::Serial(offset) => self.serial.lock().unwrap().read_u8(offset),
            MemoryRegion::Timer(offset) => self.timer_control.lock().unwrap().read_u8(offset),
            MemoryRegion::InterruptFlags => *self.interrupt_flags.lock().unwrap(),
            MemoryRegion::Sound(offset) => self.sound.lock().unwrap().read_u8(offset),
            MemoryRegion::WaveformRam(offset) => self.sound.lock().unwrap().read_u8_from_waveform(offset),
            MemoryRegion::Lcd(offset) => self.lcd.lock().unwrap().read_u8(offset),
            MemoryRegion::Key1Flag => 0xff, // Undocumented flag, KEY1 in CGB
            MemoryRegion::HighRam(offset) => self.high_ram.lock().unwrap()[offset as usize],
            MemoryRegion::InterruptEnable => *self.interrupt_enable.lock().unwrap() as u8,
        }
    }

    pub fn read_u16(&mut self, address: u16) -> u16 {
        let bytes = self.read_mem(address, 2);
        u16::from_le_bytes(bytes.try_into().unwrap_or_else(|v: Vec<u8>| {
            panic!(
                "Tried to get 2 bytes, but somehow read {} instead!",
                v.len()
            );
        }))
    }

    pub fn read_mem(&self, address: u16, length: u16) -> Vec<u8> {
        let mut vec = Vec::with_capacity(length as usize);

        for addr in address..address + length {
            let byte = self.read_u8(addr);
            vec.push(byte);
        }

        vec
    }

    pub fn write_u8(&mut self, address: u16, byte: u8) {
        let region = MemoryRegion::from(address);
        match region {
            MemoryRegion::CartridgeBank0(offset) => {
                self.cartridge.lock().unwrap().write_rom_bank_0(offset, byte)
            }
            MemoryRegion::CartridgeBankSelectable(offset) => self
                .cartridge
                .lock().unwrap()
                .write_rom_selected_bank(offset, byte),
            MemoryRegion::VideoRam(offset) => self.ppu.lock().unwrap().write_video_ram(offset, byte),
            MemoryRegion::ExternalRam(offset) => self
                .cartridge
                .lock().unwrap()
                .write_to_external_ram(offset, byte),
            MemoryRegion::WorkRam(offset) => self.ram.lock().unwrap()[offset as usize] = byte,
            MemoryRegion::ObjectAttributeMemory(offset) => self
                .ppu
                .lock().unwrap()
                .write_object_attribute_memory(offset, byte),
            MemoryRegion::Unused => (),
            MemoryRegion::Joypad => self.joypad.lock().unwrap().write_u8(byte),
            MemoryRegion::Serial(offset) => self.serial.lock().unwrap().write_u8(offset, byte),
            MemoryRegion::Timer(offset) => self.timer_control.lock().unwrap().write_u8(offset, byte),
            MemoryRegion::InterruptFlags => *self.interrupt_flags.lock().unwrap() = byte,
            MemoryRegion::Sound(offset) => self.sound.lock().unwrap().write_u8(offset, byte),
            MemoryRegion::WaveformRam(offset) => {
                self.sound.lock().unwrap().write_u8_from_waveform(offset, byte)
            }
            MemoryRegion::Lcd(offset) => self.lcd.lock().unwrap().write_u8(offset, byte),
            MemoryRegion::Key1Flag => (),
            MemoryRegion::HighRam(offset) => self.high_ram.lock().unwrap()[offset as usize] = byte,
            MemoryRegion::InterruptEnable => {
                // println!("Setting IE to {}", byte);
                *self.interrupt_enable.lock().unwrap() = byte
            }
        }
    }

    pub fn write_u16(&mut self, address: u16, v: u16) {
        self.write_mem(address, &v.to_le_bytes()[..]);
    }

    pub fn write_mem(&mut self, address: u16, bytes: &[u8]) {
        for (i, byte) in bytes.iter().enumerate() {
            self.write_u8(address + i as u16, *byte);
        }
    }
}
