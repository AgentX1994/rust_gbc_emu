use std::convert::From;

use crate::gbc::ppu::{ColorIndex, TileAddressingMethod};
use crate::gbc::utils::Flag;

#[derive(Copy, Clone, Debug)]
pub enum Color {
    White,
    LightGray,
    DarkGray,
    Black,
}

impl Default for Color {
    fn default() -> Self {
        Self::White
    }
}

impl From<u8> for Color {
    fn from(x: u8) -> Self {
        match x {
            0 => Self::White,
            1 => Self::LightGray,
            2 => Self::DarkGray,
            3 => Self::Black,
            _ => unreachable!(),
        }
    }
}

impl Color {
    fn to_u8(self) -> u8 {
        match self {
            Color::White => 0,
            Color::LightGray => 1,
            Color::DarkGray => 2,
            Color::Black => 3,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Palette {
    pub colors: [Color; 4],
}

impl From<u8> for Palette {
    fn from(v: u8) -> Self {
        Self {
            colors: [
                Color::from(v & 0x3),
                Color::from((v >> 2) & 0x3),
                Color::from((v >> 4) & 0x3),
                Color::from((v >> 6) & 0x3),
            ],
        }
    }
}

impl Palette {
    fn to_u8(self) -> u8 {
        (self.colors[3].to_u8() << 6)
            | (self.colors[2].to_u8() << 4)
            | (self.colors[1].to_u8() << 2)
            | self.colors[0].to_u8()
    }

    #[must_use]
    pub fn get_color(&self, index: &ColorIndex) -> Color {
        match index {
            ColorIndex::Color0 => self.colors[0],
            ColorIndex::Color1 => self.colors[1],
            ColorIndex::Color2 => self.colors[2],
            ColorIndex::Color3 => self.colors[3],
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TileMap {
    From9800,
    From9C00,
}

impl From<bool> for TileMap {
    fn from(v: bool) -> Self {
        if v {
            Self::From9C00
        } else {
            Self::From9800
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
pub enum SpriteSize {
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
struct Control {
    enable: Flag,
    window_tile_map: TileMap,
    window_enable: Flag,
    tile_addressing_mode: TileAddressingMethod,
    bg_tile_map: TileMap,
    sprite_size: SpriteSize,
    sprite_enable: Flag,
    bg_window_enable_or_priority: Flag,
}

impl From<u8> for Control {
    fn from(v: u8) -> Self {
        Self {
            enable: ((v >> 7) == 1).into(),
            window_tile_map: (((v >> 6) & 1) == 1).into(),
            window_enable: (((v >> 5) & 1) == 1).into(),
            tile_addressing_mode: (((v >> 4) & 1) == 1).into(),
            bg_tile_map: (((v >> 3) & 1) == 1).into(),
            sprite_size: (((v >> 2) & 1) == 1).into(),
            sprite_enable: (((v >> 1) & 1) == 1).into(),
            bg_window_enable_or_priority: ((v & 1) == 1).into(),
        }
    }
}

impl From<Control> for u8 {
    fn from(lcd: Control) -> Self {
        let mut x = 0_u8;
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
struct Status {
    interrupt_on_lyc: Flag,
    interrupt_on_oam: Flag,
    interrupt_on_vblank: Flag,
    interrupt_on_hblank: Flag,
    lyc_compare_type: LycCompareType,
    mode: LcdStatusMode,
}

impl From<u8> for Status {
    fn from(v: u8) -> Self {
        Self {
            interrupt_on_lyc: (((v >> 6) & 1) == 1).into(),
            interrupt_on_oam: (((v >> 5) & 1) == 1).into(),
            interrupt_on_vblank: (((v >> 4) & 1) == 1).into(),
            interrupt_on_hblank: (((v >> 3) & 1) == 1).into(),
            lyc_compare_type: (((v >> 2) & 1) == 1).into(),
            mode: (v & 0x3).into(),
        }
    }
}

impl From<Status> for u8 {
    fn from(status: Status) -> Self {
        let mut x = 0_u8;
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
    control: Control,
    status: Status,
    scroll_y: u8,
    scroll_x: u8,
    ly: u8,
    ly_compare: u8,
    dma_start_high_byte: u8,
    background_palette: Palette,
    object_palette_0: Palette,
    object_pallete_1: Palette,
    window_y: u8,
    window_x: u8,
    lx: i16,
    last_stat_interrupt: bool,
    dma_running: bool,
    dma_low_byte: u8,
}

impl Default for Lcd {
    fn default() -> Self {
        Self {
            control: 0x91.into(),
            status: 0x85.into(),
            scroll_y: 0,
            scroll_x: 0,
            ly: 0,
            ly_compare: 0,
            dma_start_high_byte: 0xff,
            background_palette: 0xfc.into(),
            object_palette_0: 0xff.into(),
            object_pallete_1: 0xff.into(),
            window_y: 0,
            window_x: 0,
            lx: -80,
            last_stat_interrupt: true,
            dma_running: false,
            dma_low_byte: 0,
        }
    }
}

impl Lcd {
    #[must_use]
    pub fn read_u8(&self, offset: u16) -> u8 {
        match offset {
            0x0 => self.control.into(),
            0x1 => self.status.into(),
            0x2 => self.scroll_y,
            0x3 => self.scroll_x,
            0x4 => self.ly,
            0x5 => self.ly_compare,
            0x6 => self.dma_start_high_byte,
            0x7 => self.background_palette.to_u8(),
            0x8 => self.object_palette_0.to_u8(),
            0x9 => self.object_pallete_1.to_u8(),
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
            0x6 => {
                self.dma_start_high_byte = byte;
                self.dma_low_byte = 0;
                self.dma_running = true;
            }
            0x7 => self.background_palette = byte.into(),
            0x8 => self.object_palette_0 = byte.into(),
            0x9 => self.object_pallete_1 = byte.into(),
            0xa => self.window_y = byte,
            0xb => self.window_x = byte,
            _ => unreachable!(),
        }
    }

    #[must_use]
    pub fn tick(&mut self) -> (bool, bool) {
        let mut vblank_interrupt = false;
        let mut stat_interrupt = false;
        self.lx += 1;
        if self.lx > 376 {
            self.ly += 1;
            self.lx = 0;
        }
        if self.ly == 144 {
            vblank_interrupt = true;
        } else if self.ly == 154 {
            self.ly = 0;
        }

        if self.status.interrupt_on_hblank.to_bool() && self.lx == 160 {
            stat_interrupt = true;
        }
        if self.status.interrupt_on_vblank.to_bool() && self.ly == 144 {
            stat_interrupt = true;
        }
        // TODO OAM stat interrupt
        if self.status.interrupt_on_lyc.to_bool() && self.ly == self.ly_compare {
            stat_interrupt = true;
        }

        // TODO Deal with modes

        // only interrupt on rising edge
        let should_stat_interrupt = self.last_stat_interrupt != stat_interrupt;
        self.last_stat_interrupt = stat_interrupt;
        (vblank_interrupt, should_stat_interrupt)
    }

    #[must_use]
    pub fn get_lcd_enable(&self) -> bool {
        self.control.enable.to_bool()
    }

    #[must_use]
    pub fn get_ly(&self) -> u8 {
        self.ly
    }

    #[must_use]
    pub fn get_lx(&self) -> i16 {
        self.lx
    }

    #[must_use]
    pub fn get_scroll_offsets(&self) -> (u8, u8) {
        (self.scroll_x, self.scroll_y)
    }

    #[must_use]
    pub fn get_background_palette(&self) -> Palette {
        self.background_palette
    }

    #[must_use]
    pub fn get_addressing_mode(&self) -> TileAddressingMethod {
        self.control.tile_addressing_mode
    }

    #[must_use]
    pub fn get_background_tile_map(&self) -> TileMap {
        self.control.bg_tile_map
    }

    #[must_use]
    pub fn get_background_window_priority(&self) -> bool {
        self.control.bg_window_enable_or_priority.to_bool()
    }

    #[must_use]
    pub fn get_sprite_enable(&self) -> bool {
        self.control.sprite_enable.to_bool()
    }

    #[must_use]
    pub fn get_sprite_size(&self) -> SpriteSize {
        self.control.sprite_size
    }

    #[must_use]
    pub fn get_object_palettes(&self) -> (Palette, Palette) {
        (self.object_palette_0, self.object_pallete_1)
    }

    #[must_use]
    pub fn get_window_enable(&self) -> bool {
        self.control.window_enable.to_bool()
    }

    #[must_use]
    pub fn get_window_coords(&self) -> (u8, u8) {
        (self.window_x, self.window_y)
    }

    #[must_use]
    pub fn get_window_tile_map(&self) -> TileMap {
        self.control.window_tile_map
    }

    #[must_use]
    pub fn get_dma_running(&self) -> bool {
        self.dma_running
    }

    #[must_use]
    pub fn get_dma_addresses(&self) -> Option<(u16, u16)> {
        if self.dma_running {
            let source = (u16::from(self.dma_start_high_byte) << 8) | u16::from(self.dma_low_byte);
            let destination = 0xfe00 | u16::from(self.dma_low_byte);
            Some((source, destination))
        } else {
            None
        }
    }

    pub fn tick_dma(&mut self) {
        self.dma_low_byte += 1;
        if self.dma_low_byte == 0xa0 {
            self.dma_running = false;
        }
    }
}
