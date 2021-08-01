use std::convert::TryInto;

use super::cartridge::Cartridge;
use super::mmio::Mmio;

enum MemoryRegion {
    CartridgeBank0(u16),
    CartridgeBankSelectable(u16),
    VideoRam(u16),
    ExternalRam(u16),
    WorkRam(u16),
    ObjectAttributeMemory(u16),
    Unused,
    Mmio(u16),
    HighRam(u16),
    InterruptMasterEnable,
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
            0xff00..=0xff7f => MemoryRegion::Mmio(address - 0xff00),
            0xff80..=0xfffe => MemoryRegion::HighRam(address - 0xff80),
            0xffff => MemoryRegion::InterruptMasterEnable,
        }
    }
}

#[derive(Debug)]
pub struct MemoryBus {
    pub cartridge: Cartridge,
    pub ram: [u8; 8192],
    pub mmio: Mmio,
    pub high_ram: [u8; 126],
    pub interrupt_master_enable: u8,
}

impl MemoryBus {
    pub fn new(cartridge: Cartridge) -> Self {
        MemoryBus {
            cartridge,
            ram: [0; 8192],
            mmio: Mmio::default(),
            high_ram: [0; 126],
            interrupt_master_enable: 0,
        }
    }

    pub fn read_u8(&self, address: u16) -> u8 {
        let region = MemoryRegion::from(address);
        match region {
            MemoryRegion::CartridgeBank0(offset) => self.cartridge.read_rom_bank_0(offset),
            MemoryRegion::CartridgeBankSelectable(offset) => {
                self.cartridge.read_rom_selected_bank(offset)
            }
            MemoryRegion::VideoRam(offset) => todo!(),
            MemoryRegion::ExternalRam(offset) => self.cartridge.read_from_external_ram(offset),
            MemoryRegion::WorkRam(offset) => self.ram[offset as usize],
            MemoryRegion::ObjectAttributeMemory(offset) => todo!(),
            MemoryRegion::Unused => {
                // Use Color Game Boy Revision E behavior I guess?
                let second_nibble = ((address >> 4) & 0xf) as u8;
                (second_nibble << 4) | second_nibble
            }
            MemoryRegion::Mmio(offset) => self.mmio.read_u8(offset),
            MemoryRegion::HighRam(offset) => self.high_ram[offset as usize],
            MemoryRegion::InterruptMasterEnable => self.interrupt_master_enable,
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
            MemoryRegion::CartridgeBank0(offset) => todo!(),
            MemoryRegion::CartridgeBankSelectable(offset) => todo!(),
            MemoryRegion::VideoRam(offset) => todo!(),
            MemoryRegion::ExternalRam(offset) => self.cartridge.write_to_external_ram(offset, byte),
            MemoryRegion::WorkRam(offset) => self.ram[offset as usize] = byte,
            MemoryRegion::ObjectAttributeMemory(offset) => todo!(),
            MemoryRegion::Unused => todo!(),
            MemoryRegion::Mmio(offset) => self.mmio.write_u8(offset, byte),
            MemoryRegion::HighRam(offset) => self.high_ram[offset as usize] = byte,
            MemoryRegion::InterruptMasterEnable => self.interrupt_master_enable = byte,
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
