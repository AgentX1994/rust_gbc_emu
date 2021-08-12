use std::{convert::{TryFrom, TryInto}, fs::File, io::{self, Read}, path::{Path, PathBuf}};

const NINTENDO_LOGO_BYTES: [u8; 0x30] = [
    0xce, 0xed, 0x66, 0x66, 0xcc, 0x0d, 0x00, 0x0b, 0x03, 0x73, 0x00, 0x83, 0x00, 0x0c, 0x00, 0x0d,
    0x00, 0x08, 0x11, 0x1f, 0x88, 0x89, 0x00, 0x0e, 0xdc, 0xcc, 0x6e, 0xe6, 0xdd, 0xdd, 0xd9, 0x99,
    0xbb, 0xbb, 0x67, 0x63, 0x6e, 0x0e, 0xec, 0xcc, 0xdd, 0xdc, 0x99, 0x9f, 0xbb, 0xb9, 0x33, 0x3e,
];

#[derive(Debug)]
pub enum CartridgeType {
    Rom = 0x00,
    Mbc1 = 0x01,
    Mbc1Ram = 0x02,
    Mbc1RamBattery = 0x03,
    Mbc2 = 0x05,
    Mbc2Battery = 0x06,
    RomRam = 0x08,
    RomRamBattery = 0x09,
    Mmm01 = 0x0b,
    Mmm01Ram = 0x0c,
    Mmm01RamBattery = 0x0d,
    Mbc3TimerBattery = 0x0f,
    Mbc3TimerRamBattery = 0x10,
    Mbc3 = 0x11,
    Mbc3Ram = 0x12,
    Mbc3RamBattery = 0x13,
    Mbc5 = 0x19,
    Mbc5Ram = 0x1a,
    Mbc5RamBattery = 0x1b,
    Mbc5Rumble = 0x1c,
    Mbc5RumbleRam = 0x1d,
    Mbc5RumbleRamBattery = 0x1e,
    Mbc6 = 0x20,
    Mbc7SensorRumbleRamBattery = 0x22,
    PocketCamera = 0xfc,
    BandaiTama5 = 0xfd,
    HuC3 = 0xfe,
    Huc1RamBattery = 0xff,
}

impl Default for CartridgeType {
    fn default() -> Self {
        CartridgeType::Rom
    }
}

impl CartridgeType {
    fn mbc_type(&self) -> u8 {
        match self {
            CartridgeType::Mbc1 | CartridgeType::Mbc1Ram | CartridgeType::Mbc1RamBattery => 1,
            CartridgeType::Mbc2 | CartridgeType::Mbc2Battery => 2,
            CartridgeType::Mbc3
            | CartridgeType::Mbc3Ram
            | CartridgeType::Mbc3RamBattery
            | CartridgeType::Mbc3TimerBattery
            | CartridgeType::Mbc3TimerRamBattery => 3,
            CartridgeType::Mbc5
            | CartridgeType::Mbc5Ram
            | CartridgeType::Mbc5RamBattery
            | CartridgeType::Mbc5Rumble
            | CartridgeType::Mbc5RumbleRam
            | CartridgeType::Mbc5RumbleRamBattery => 5,
            CartridgeType::Mbc6 => 6,
            CartridgeType::Mbc7SensorRumbleRamBattery => 7,
            _ => 0,
        }
    }
}

impl TryFrom<u8> for CartridgeType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == CartridgeType::Rom as u8 => Ok(CartridgeType::Rom),
            x if x == CartridgeType::Mbc1 as u8 => Ok(CartridgeType::Mbc1),
            x if x == CartridgeType::Mbc1Ram as u8 => Ok(CartridgeType::Mbc1Ram),
            x if x == CartridgeType::Mbc1RamBattery as u8 => Ok(CartridgeType::Mbc1RamBattery),
            x if x == CartridgeType::Mbc2 as u8 => Ok(CartridgeType::Mbc2),
            x if x == CartridgeType::Mbc2Battery as u8 => Ok(CartridgeType::Mbc2Battery),
            x if x == CartridgeType::RomRam as u8 => Ok(CartridgeType::RomRam),
            x if x == CartridgeType::RomRamBattery as u8 => Ok(CartridgeType::RomRamBattery),
            x if x == CartridgeType::Mmm01 as u8 => Ok(CartridgeType::Mmm01),
            x if x == CartridgeType::Mmm01Ram as u8 => Ok(CartridgeType::Mmm01Ram),
            x if x == CartridgeType::Mmm01RamBattery as u8 => Ok(CartridgeType::Mmm01RamBattery),
            x if x == CartridgeType::Mbc3TimerBattery as u8 => Ok(CartridgeType::Mbc3TimerBattery),
            x if x == CartridgeType::Mbc3TimerRamBattery as u8 => {
                Ok(CartridgeType::Mbc3TimerRamBattery)
            }
            x if x == CartridgeType::Mbc3 as u8 => Ok(CartridgeType::Mbc3),
            x if x == CartridgeType::Mbc3Ram as u8 => Ok(CartridgeType::Mbc3Ram),
            x if x == CartridgeType::Mbc3RamBattery as u8 => Ok(CartridgeType::Mbc3RamBattery),
            x if x == CartridgeType::Mbc5 as u8 => Ok(CartridgeType::Mbc5),
            x if x == CartridgeType::Mbc5Ram as u8 => Ok(CartridgeType::Mbc5Ram),
            x if x == CartridgeType::Mbc5RamBattery as u8 => Ok(CartridgeType::Mbc5RamBattery),
            x if x == CartridgeType::Mbc5Rumble as u8 => Ok(CartridgeType::Mbc5Rumble),
            x if x == CartridgeType::Mbc5RumbleRam as u8 => Ok(CartridgeType::Mbc5RumbleRam),
            x if x == CartridgeType::Mbc5RumbleRamBattery as u8 => {
                Ok(CartridgeType::Mbc5RumbleRamBattery)
            }
            x if x == CartridgeType::Mbc6 as u8 => Ok(CartridgeType::Mbc6),
            x if x == CartridgeType::Mbc7SensorRumbleRamBattery as u8 => {
                Ok(CartridgeType::Mbc7SensorRumbleRamBattery)
            }
            x if x == CartridgeType::PocketCamera as u8 => Ok(CartridgeType::PocketCamera),
            x if x == CartridgeType::BandaiTama5 as u8 => Ok(CartridgeType::BandaiTama5),
            x if x == CartridgeType::HuC3 as u8 => Ok(CartridgeType::HuC3),
            x if x == CartridgeType::Huc1RamBattery as u8 => Ok(CartridgeType::Huc1RamBattery),
            _ => Err(()),
        }
    }
}

fn calculate_header_checksum(header: &[u8]) -> u8 {
    assert!(header.len() > 0x4d);
    let mut x: u8 = 0;
    for i in 0x34..0x4d {
        x = x.wrapping_sub(header[i]).wrapping_sub(1)
    }
    x
}

fn calculate_global_checksum(rom: &[u8]) -> u16 {
    let mut x: u16 = 0;
    for i in 0..rom.len() {
        if i == 0x14e || i == 0x14f {
            continue;
        }
        x = x.wrapping_add(rom[i] as u16);
    }

    x
}

#[derive(Debug)]
pub enum GameBoyColorSupport {
    NoColorSupport,
    SupportsColor,
    OnlyColor
}

impl Default for GameBoyColorSupport {
    fn default() -> Self {
        Self::NoColorSupport
    }
}

#[derive(Debug, Default)]
pub struct Cartridge {
    pub rom_path: PathBuf,
    pub rom: Vec<u8>,
    pub title: String,
    pub manufacturer_code: [u8; 4],
    pub color_support: GameBoyColorSupport,
    pub licensee_code: [u8; 2],
    pub supports_sgb: bool,
    pub cartridge_type: CartridgeType,
    pub rom_size: u32,
    pub ram_size: u32,
    pub is_japanese: bool,
    pub rom_version: u8,
    pub header_checksum: u8,
    pub global_checksum: u16,
    pub ram: Vec<u8>,
    pub enable_external_ram: bool,
    pub rom_bank_selected: u8,
    pub advanced_banking_mode: bool,
}

impl Cartridge {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let rom_path = path.as_ref().to_owned();
        let mut file = File::open(path)?;
        let mut rom = Vec::<u8>::new();
        file.read_to_end(&mut rom)?;

        // Read header
        let header = &rom[0x100..0x150];

        // Check Nintendo Logo
        // This isn't a fatal error for an emulator
        let nintendo_logo = &header[0x04..0x34];
        if nintendo_logo != NINTENDO_LOGO_BYTES {
            eprintln!("Nintendo Logo doesn't match!");
        } else {
            println!("Nintendo Logo matches!");
        }

        // is this a color cartridge?
        let title: String;
        let manufacturer_code: [u8; 4];
        let color_support: GameBoyColorSupport;
        if header[0x43] & 0x80 != 0 {
            color_support = if header[0x43] & 0xc0 == 0xc0 {
                GameBoyColorSupport::OnlyColor
            } else {
                GameBoyColorSupport::SupportsColor
            };
            // Not sure how to tell whether the title is 11 or 15 bytes...
            manufacturer_code = header[0x3f..0x43]
                .try_into()
                .expect("slice with incorrect length");
            title = String::from_utf8_lossy(&header[0x34..0x43]).into();
        } else {
            color_support = GameBoyColorSupport::NoColorSupport;
            manufacturer_code = [0, 0, 0, 0];
            title = String::from_utf8_lossy(&header[0x34..0x44]).into();
        }
        let licensee_code: [u8; 2] = {
            let old_licensee_code = header[0x4b];
            if old_licensee_code == 0x33 {
                header[0x44..0x46]
                    .try_into()
                    .expect("slice with incorrect length v2")
            } else {
                [old_licensee_code, 0]
            }
        };
        let supports_sgb: bool = header[0x46] == 0x3;
        let cartridge_type: CartridgeType =
            header[0x47].try_into().expect("Invalid cartridge type!");
        let rom_size: u32 = (32 * 1024) << header[0x48];
        let ram_size_code = header[0x49];
        let ram_size: u32 = if cartridge_type.mbc_type() == 2 {
            if ram_size_code != 0 {
                eprintln!("Error: Cartridge uses MBC2 but ram size code is not 0!");
            }
            256
        } else {
            match ram_size_code {
                0 => 0,
                2 => 8 * 1024,
                3 => 32 * 1024,
                4 => 128 * 1024,
                5 => 64 * 1024,
                _ => panic!("Unknown ram size code {}!", ram_size_code),
            }
        };
        let is_japanese = header[0x4a] == 0;
        let rom_version = header[0x4c];
        let header_checksum = header[0x4d];
        let calculated_checksum = calculate_header_checksum(header);
        if header_checksum != calculated_checksum {
            eprintln!(
                "Error: header checksum doesn't match! header contains {}, but calculated {}",
                header_checksum, calculated_checksum
            );
        }
        let global_checksum: u16 = ((header[0x4e] as u16) << 8) | header[0x4f] as u16;
        let calculated_global_checksum = calculate_global_checksum(&rom[..]);
        if global_checksum != calculated_global_checksum {
            eprintln!(
                "Error: global checksum doesn't match! header contains {}, but calculated {}",
                global_checksum, calculated_global_checksum
            );
        }

        Ok(Cartridge {
            rom_path,
            rom,
            title,
            manufacturer_code,
            color_support,
            licensee_code,
            supports_sgb,
            cartridge_type,
            rom_size,
            ram_size,
            is_japanese,
            rom_version,
            header_checksum,
            global_checksum,
            ram: vec![0; ram_size as usize],
            enable_external_ram: false,
            rom_bank_selected: 1,
            advanced_banking_mode: false,
        })
    }

    pub fn read_rom_bank_0(&self, offset: u16) -> u8 {
        assert!(offset < 16384);
        self.rom[offset as usize]
    }

    pub fn write_rom_bank_0(&mut self, offset: u16, byte: u8) {
        match offset {
            0x0000..=0x1fff => {
                // external ram enable
                self.enable_external_ram = (byte & 0xf) == 0x0a;
            }
            0x2000..=0x3fff => {
                // rom bank switch
                // TODO Check MBC implementation
                self.rom_bank_selected = byte & 0x1f;
                if self.rom_bank_selected == 0 {
                    self.rom_bank_selected = 1; // Don't select bank 0 again
                }
                let available_banks = (self.rom_size / 16384) as u8;
                self.rom_bank_selected &= available_banks - 1;
            }
            _ => unreachable!()
        }
    }

    pub fn read_rom_selected_bank(&self, offset: u16) -> u8 {
        assert!(offset < 16384);
        // TODO check MBC implementation
        let address = offset + 16384*self.rom_bank_selected as u16;
        self.rom[address as usize]
    }

    pub fn write_rom_selected_bank(&mut self, offset: u16, byte: u8) {
        match offset {
            0x0000..=0x1fff => {
                if !self.advanced_banking_mode {
                    self.rom_bank_selected = ((byte & 0x3) << 5) | (self.rom_bank_selected | 0x1f);
                    let available_banks = (self.rom_size / 16384) as u8;
                    self.rom_bank_selected &= available_banks - 1;
                } else {
                    todo!()
                }
            }
            0x2000..=0x3fff => {
                self.advanced_banking_mode = (byte & 1) != 0;
            }
            _ => unreachable!()
        }
    }

    pub fn read_from_external_ram(&self, offset: u16) -> u8 {
        if !self.enable_external_ram {
            return 0xff;
        }
        if (offset as usize) < self.ram.len() {
            self.ram[offset as usize]
        } else {
            0x00
        }
    }

    pub fn write_to_external_ram(&mut self, offset: u16, v: u8) {
        if self.enable_external_ram {
            if (offset as usize) < self.ram.len() {
                self.ram[offset as usize] = v;
            }
        }
    }
}
