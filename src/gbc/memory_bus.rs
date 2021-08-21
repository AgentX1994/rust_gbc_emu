use super::cartridge::Cartridge;
use super::debug::{AccessType, Breakpoint};
use super::mmio::{apu::Sound, joypad::Joypad, lcd::Lcd, serial::Comms, timer::Timer};
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
    BootRomDisable,
    HighRam(u16),
    InterruptEnable,
}

impl From<u16> for MemoryRegion {
    fn from(address: u16) -> Self {
        #![allow(clippy::match_same_arms)]
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
            0xff50 => MemoryRegion::BootRomDisable,
            0xff80..=0xfffe => MemoryRegion::HighRam(address - 0xff80),
            0xffff => MemoryRegion::InterruptEnable,
            _ => MemoryRegion::Unused,
        }
    }
}

#[derive(Debug)]
pub struct MemoryBus {
    pub cartridge: Cartridge,
    pub ram: [u8; 8192],
    pub ppu: PictureProcessingUnit,
    pub joypad: Joypad,
    pub serial: Comms,
    pub timer_control: Timer,
    pub sound: Sound,
    pub lcd: Lcd,
    pub boot_rom_disable: u8,
    pub vram_select: u8,
    pub disable_boot_rom: bool,
    pub vram_dma: [u8; 4],
    pub color_palettes: [u8; 2],
    pub wram_bank_select: u8,
    pub interrupt_flags: u8,
    pub high_ram: [u8; 127],
    pub interrupt_enable: u8,
    boot_rom: &'static [u8; 256],
    last_bus_value: u8,
    memory_breakpoints: Vec<Breakpoint>,
    break_reason: Option<Breakpoint>,
}

impl MemoryBus {
    #[must_use]
    pub fn new(cartridge: Cartridge) -> Self {
        MemoryBus {
            cartridge,
            ram: [0; 8192],
            ppu: PictureProcessingUnit::default(),
            joypad: Joypad::default(),
            serial: Comms::default(),
            timer_control: Timer::default(),
            sound: Sound::default(),
            lcd: Lcd::default(),
            boot_rom_disable: 0,
            vram_select: 0,
            disable_boot_rom: false,
            vram_dma: [0; 4],
            color_palettes: [0; 2],
            wram_bank_select: 0,
            interrupt_flags: 0,
            high_ram: [0; 127],
            interrupt_enable: 0,
            boot_rom: include_bytes!("../../dmg_boot.bin"),
            last_bus_value: 0,
            memory_breakpoints: Vec::new(),
            break_reason: None,
        }
    }

    #[must_use]
    pub fn reset(self) -> Self {
        Self::new(self.cartridge)
    }

    pub fn add_breakpoint(&mut self, bp: Breakpoint) {
        // Only add it if it isn't just an execute breakpoint
        if matches!(bp.access_type, AccessType::Execute) {
            return;
        }
        self.memory_breakpoints.push(bp);
    }

    #[must_use]
    pub fn check_breakpoints(&self, address: u16, write: bool) -> Option<Breakpoint> {
        for bp in &self.memory_breakpoints {
            if bp.matches_address(address) && (write && bp.access_type.on_write())
                || (!write && bp.access_type.on_read())
            {
                return Some(*bp);
            }
        }
        None
    }

    #[must_use]
    pub fn get_break_reason(&mut self) -> Option<Breakpoint> {
        self.break_reason.take()
    }

    #[must_use]
    pub fn read_u8(&mut self, address: u16) -> u8 {
        let region = MemoryRegion::from(address);
        // Technically there are more is one bus and this is complicated
        if self.lcd.get_dma_running() && !matches!(region, MemoryRegion::HighRam(_)) {
            return self.last_bus_value;
        }
        self.break_reason = self
            .break_reason
            .or_else(|| self.check_breakpoints(address, false));
        // I'd like to overwrite self.last_bus_value here, but I also don't want to make this
        // a &mut self function...
        self.last_bus_value = match region {
            MemoryRegion::CartridgeBank0(offset) => {
                if self.boot_rom_disable == 0 && offset < 0x100 {
                    self.boot_rom[offset as usize]
                } else {
                    self.cartridge.read_rom_bank_0(offset)
                }
            }
            MemoryRegion::CartridgeBankSelectable(offset) => {
                self.cartridge.read_rom_selected_bank(offset)
            }
            MemoryRegion::VideoRam(offset) => self.ppu.read_video_ram(offset),
            MemoryRegion::ExternalRam(offset) => self.cartridge.read_from_external_ram(offset),
            MemoryRegion::WorkRam(offset) => self.ram[offset as usize],
            MemoryRegion::ObjectAttributeMemory(offset) => {
                self.ppu.read_object_attribute_memory(offset)
            }
            MemoryRegion::Unused => {
                // Use Color Game Boy Revision E behavior I guess?
                #[allow(clippy::cast_possible_truncation)]
                let second_nibble = ((address >> 4) & 0xf) as u8;
                (second_nibble << 4) | second_nibble
            }
            MemoryRegion::Joypad => self.joypad.read_u8(),
            MemoryRegion::Serial(offset) => self.serial.read_u8(offset),
            MemoryRegion::Timer(offset) => self.timer_control.read_u8(offset),
            MemoryRegion::InterruptFlags => self.interrupt_flags,
            MemoryRegion::Sound(offset) => self.sound.read_u8(offset),
            MemoryRegion::WaveformRam(offset) => self.sound.read_u8_from_waveform(offset),
            MemoryRegion::Lcd(offset) => self.lcd.read_u8(offset),
            MemoryRegion::BootRomDisable => self.boot_rom_disable,
            MemoryRegion::Key1Flag => 0xff, // Undocumented flag, KEY1 in CGB
            MemoryRegion::HighRam(offset) => self.high_ram[offset as usize],
            MemoryRegion::InterruptEnable => self.interrupt_enable as u8,
        };
        self.last_bus_value
    }

    #[must_use]
    pub fn read_u16(&mut self, address: u16) -> u16 {
        let byte1 = self.read_u8(address);
        let byte2 = self.read_u8(address.wrapping_add(1));
        (u16::from(byte2) << 8) | u16::from(byte1)
    }

    #[must_use]
    pub fn read_mem(&mut self, address: u16, length: u16) -> Vec<u8> {
        let mut vec = Vec::with_capacity(length as usize);

        for addr in address..address + length {
            let byte = self.read_u8(addr);
            vec.push(byte);
        }

        vec
    }

    pub fn write_u8(&mut self, address: u16, byte: u8) {
        #![allow(clippy::match_same_arms)]
        let region = MemoryRegion::from(address);
        // Technically there are more is one bus and this is complicated
        if self.lcd.get_dma_running() && !matches!(region, MemoryRegion::HighRam(_)) {
            return;
        }
        self.break_reason = self
            .break_reason
            .or_else(|| self.check_breakpoints(address, true));
        match region {
            MemoryRegion::CartridgeBank0(offset) => self.cartridge.write_rom_bank_0(offset, byte),
            MemoryRegion::CartridgeBankSelectable(offset) => {
                self.cartridge.write_rom_selected_bank(offset, byte);
            }
            MemoryRegion::VideoRam(offset) => self.ppu.write_video_ram(offset, byte),
            MemoryRegion::ExternalRam(offset) => self.cartridge.write_to_external_ram(offset, byte),
            MemoryRegion::WorkRam(offset) => self.ram[offset as usize] = byte,
            MemoryRegion::ObjectAttributeMemory(offset) => {
                self.ppu.write_object_attribute_memory(offset, byte);
            }
            MemoryRegion::Unused => (),
            MemoryRegion::Joypad => self.joypad.write_u8(byte),
            MemoryRegion::Serial(offset) => self.serial.write_u8(offset, byte),
            MemoryRegion::Timer(offset) => self.timer_control.write_u8(offset, byte),
            MemoryRegion::InterruptFlags => self.interrupt_flags = byte,
            MemoryRegion::Sound(offset) => self.sound.write_u8(offset, byte),
            MemoryRegion::WaveformRam(offset) => self.sound.write_u8_from_waveform(offset, byte),
            MemoryRegion::Lcd(offset) => self.lcd.write_u8(offset, byte),
            MemoryRegion::BootRomDisable => self.boot_rom_disable = byte,
            MemoryRegion::Key1Flag => (),
            MemoryRegion::HighRam(offset) => self.high_ram[offset as usize] = byte,
            MemoryRegion::InterruptEnable => self.interrupt_enable = byte,
        }
        self.last_bus_value = byte;
    }

    pub fn write_u16(&mut self, address: u16, v: u16) {
        self.write_mem(address, &v.to_le_bytes()[..]);
    }

    pub fn write_mem(&mut self, address: u16, bytes: &[u8]) {
        for (i, byte) in bytes.iter().enumerate() {
            #[allow(clippy::cast_possible_truncation)]
            self.write_u8(address.wrapping_add(i as u16), *byte);
        }
    }

    pub fn run_dma(&mut self, cycles: u64) {
        if !self.lcd.get_dma_running() {
            return;
        }

        for _ in 0..cycles {
            let (source, destination) = match self.lcd.get_dma_addresses() {
                Some(v) => v,
                None => break,
            };

            let v = self.read_u8(source);
            self.last_bus_value = v;
            self.write_u8(destination, v);

            self.lcd.tick_dma();
        }
    }
}
