use std::convert::From;

use crate::gbc::ppu::TileAddressingMethod;

#[derive(Copy, Clone, Debug)]
enum TileMap {
    From9800,
    From9C00,
}

impl From<bool> for TileMap {
    fn from(v: bool) -> Self {
        if v {
            Self::From9800
        } else {
            Self::From9C00
        }
    }
}

impl From<TileMap> for bool {
    fn from(s: TileMap) -> Self {
        match s {
            TileMap::From9800 => false,
            TileMap::From9C00 => true,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum SpriteSize {
    Small, // 8 x 8
    Large, // 8 x 16
}

impl From<bool> for SpriteSize {
    fn from(v: bool) -> Self {
        if v {
            Self::Small
        } else {
            Self::Large
        }
    }
}

impl From<SpriteSize> for bool {
    fn from(s: SpriteSize) -> Self {
        match s {
            SpriteSize::Small => false,
            SpriteSize::Large => true,
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct LcdControl {
    enable: bool,
    window_tile_map: TileMap,
    window_enable: bool,
    tile_addressing_mode: TileAddressingMethod,
    bg_tile_map: TileMap,
    sprite_size: SpriteSize,
    sprite_enable: bool,
    bg_window_enable_or_priority: bool,
}

impl From<u8> for LcdControl {
    fn from(v: u8) -> Self {
        Self {
            enable: (v >> 7) == 1,
            window_tile_map: (((v >> 6) & 1) == 1).into(),
            window_enable: ((v >> 5) & 1) == 1,
            tile_addressing_mode: (((v >> 4) & 1) == 1).into(),
            bg_tile_map: (((v >> 3) & 1) == 1).into(),
            sprite_size: (((v >> 2) & 1) == 1).into(),
            sprite_enable: ((v >> 1) & 1) == 1,
            bg_window_enable_or_priority: (v & 1) == 1,
        }
    }
}

impl From<LcdControl> for u8 {
    fn from(lcd: LcdControl) -> Self {
        let mut x = 0u8;
        x |= lcd.bg_window_enable_or_priority as u8;
        x <<= 1;
        x |= lcd.sprite_enable as u8;
        x <<= 1;
        x |= bool::from(lcd.sprite_size) as u8;
        x <<= 1;
        x |= bool::from(lcd.bg_tile_map) as u8;
        x <<= 1;
        x |= bool::from(lcd.tile_addressing_mode) as u8;
        x <<= 1;
        x |= lcd.window_enable as u8;
        x <<= 1;
        x |= bool::from(lcd.window_tile_map) as u8;
        x <<= 1;
        x |= lcd.enable as u8;
        x
    }
}

#[derive(Copy, Clone, Debug)]
enum LycCompareType {
    NotEqual,
    Equal,
}

impl From<bool> for LycCompareType {
    fn from(v: bool) -> Self {
        if v {
            Self::Equal
        } else {
            Self::NotEqual
        }
    }
}

impl From<LycCompareType> for bool {
    fn from(s: LycCompareType) -> Self {
        match s {
            LycCompareType::NotEqual => false,
            LycCompareType::Equal => true,
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum LcdStatusMode {
    InHBlank,
    InVBlank,
    SearchingOam,
    TransferringDataToLcd,
}

impl From<u8> for LcdStatusMode {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::InHBlank,
            1 => Self::InVBlank,
            2 => Self::SearchingOam,
            3 => Self::TransferringDataToLcd,
            _ => unreachable!(),
        }
    }
}

impl From<LcdStatusMode> for u8 {
    fn from(mode: LcdStatusMode) -> Self {
        match mode {
            LcdStatusMode::InHBlank => 0,
            LcdStatusMode::InVBlank => 1,
            LcdStatusMode::SearchingOam => 2,
            LcdStatusMode::TransferringDataToLcd => 3,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct LcdStatus {
    interrupt_on_lyc: bool,
    interrupt_on_oam: bool,
    interrupt_on_vblank: bool,
    interrupt_on_hblank: bool,
    lyc_compare_type: LycCompareType,
    mode: LcdStatusMode,
}

impl From<u8> for LcdStatus {
    fn from(v: u8) -> Self {
        Self {
            interrupt_on_lyc: ((v >> 6) & 1) == 1,
            interrupt_on_oam: ((v >> 5) & 1) == 1,
            interrupt_on_vblank: ((v >> 4) & 1) == 1,
            interrupt_on_hblank: ((v >> 3) & 1) == 1,
            lyc_compare_type: (((v >> 2) & 1) == 1).into(),
            mode: (v & 0x3).into(),
        }
    }
}

impl From<LcdStatus> for u8 {
    fn from(status: LcdStatus) -> Self {
        let mut x = 0u8;
        x |= status.interrupt_on_lyc as u8;
        x <<= 1;
        x |= status.interrupt_on_oam as u8;
        x <<= 1;
        x |= status.interrupt_on_vblank as u8;
        x <<= 1;
        x |= status.interrupt_on_hblank as u8;
        x <<= 1;
        x |= bool::from(status.lyc_compare_type) as u8;
        x <<= 2;
        x |= u8::from(status.mode);
        x
    }
}

#[derive(Debug)]
pub struct Lcd {
    control: LcdControl,
    status: LcdStatus,
    scroll_y: u8,
    scroll_x: u8,
    ly: u8,
    ly_compare: u8,
    dma_start_high_byte: u8,
    background_palette: u8,
    object_palette_0: u8,
    object_pallete_1: u8,
    window_y: u8,
    window_x: u8,
    lx: u8,
}

impl Default for Lcd {
    fn default() -> Self {
        Self {
            control: 0.into(),
            status: 0.into(),
            scroll_y: 0,
            scroll_x: 0,
            ly: 0,
            ly_compare: 0,
            dma_start_high_byte: 0,
            background_palette: 0,
            object_palette_0: 0,
            object_pallete_1: 0,
            window_y: 0,
            window_x: 0,
            lx: 0,
        }
    }
}

impl Lcd {
    pub fn read_u8(&self, offset: u16) -> u8 {
        match offset {
            0x0 => self.control.into(),
            0x1 => self.status.into(),
            0x2 => self.scroll_y,
            0x3 => self.scroll_x,
            0x4 => self.ly,
            0x5 => self.ly_compare,
            0x6 => self.dma_start_high_byte,
            0x7 => self.background_palette,
            0x8 => self.object_palette_0,
            0x9 => self.object_pallete_1,
            0xa => self.window_y,
            0xb => self.window_x,
            _ => unreachable!(),
        }
    }

    pub fn write_u8(&mut self, offset: u16, byte: u8) {
        match offset {
            0x0 => self.control = byte.into(),
            0x1 => self.status = byte.into(),
            0x2 => self.scroll_y = byte,
            0x3 => self.scroll_x = byte,
            0x4 => (), // unwritable: self.ly = byte,
            0x5 => self.ly_compare = byte,
            0x6 => self.dma_start_high_byte = byte,
            0x7 => self.background_palette = byte,
            0x8 => self.object_palette_0 = byte,
            0x9 => self.object_pallete_1 = byte,
            0xa => self.window_y = byte,
            0xb => self.window_x = byte,
            _ => unreachable!(),
        }
    }

    pub fn tick(&mut self, cycles: u64) -> (bool, bool) {
        let mut vblank_interrupt = false;
        let mut stat_interrupt = false;
        self.lx += cycles as u8;
        if self.lx > 160 {
            self.ly += 1;
            self.lx -= 160;
        }
        if self.ly == 144 {
            vblank_interrupt = true;
        }
        if self.ly == 154 {
            self.ly = 0;
        }

        (vblank_interrupt, stat_interrupt)
    }
}
