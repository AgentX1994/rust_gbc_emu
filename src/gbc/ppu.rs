#[derive(Copy, Clone, Debug)]
pub enum Color {
    Color0,
    Color1,
    Color2,
    Color3,
}

impl Color {
    pub fn new(color: u8) -> Self {
        match color {
            0 => Self::Color0,
            1 => Self::Color1,
            2 => Self::Color2,
            3 => Self::Color3,
            _ => unreachable!(),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::Color0
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Tile {
    lines: [u8; 16],
}

impl Tile {
    pub fn deinterleave(&self) -> [Color; 64] {
        let mut colors = [Color::default(); 64];
        for i in (0..16).step_by(2) {
            let mut byte0 = self.lines[i];
            let mut byte1 = self.lines[i + 1];

            colors[i * 8 + 0] = Color::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 1] = Color::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 2] = Color::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 3] = Color::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 4] = Color::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 5] = Color::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 6] = Color::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 7] = Color::new(((byte1 & 1) << 1) | (byte0 & 1));
        }
        colors
    }
}

pub enum TileAddressingMethod {
    From8000(u8),
    From9000(i8),
}

#[derive(Debug)]
pub struct VideoRam {
    tile_block_0: [Tile; 128],
    tile_block_1: [Tile; 128],
    tile_block_2: [Tile; 128],
    background_map_0: [u8; 32 * 32],
    background_map_1: [u8; 32 * 32],
}

impl Default for VideoRam {
    fn default() -> Self {
        Self {
            tile_block_0: [Tile::default(); 128],
            tile_block_1: [Tile::default(); 128],
            tile_block_2: [Tile::default(); 128],
            background_map_0: [0; 32 * 32],
            background_map_1: [0; 32 * 32],
        }
    }
}

impl VideoRam {
    fn read(&self, offset: u16) -> u8 {
        match offset {
            0..=0x87ff => self.tile_block_0[(offset / 16) as usize].lines[(offset % 16) as usize],
            0x8800..=0x8fff => {
                let offset = offset - 0x8800;
                self.tile_block_1[(offset / 16) as usize].lines[(offset % 16) as usize]
            }
            0x9000..=0x97ff => {
                let offset = offset - 0x8800;
                self.tile_block_2[(offset / 16) as usize].lines[(offset % 16) as usize]
            }
            0x9800..=0x9bff => {
                let offset = offset - 0x9800;
                self.background_map_0[offset as usize]
            }
            0x9c00..=0x9fff => {
                let offset = offset - 0x9c00;
                self.background_map_1[offset as usize]
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, offset: u16, byte: u8) {
        match offset {
            0..=0x87ff => {
                self.tile_block_0[(offset / 16) as usize].lines[(offset % 16) as usize] = byte
            }
            0x8800..=0x8fff => {
                let offset = offset - 0x8800;
                self.tile_block_1[(offset / 16) as usize].lines[(offset % 16) as usize] = byte;
            }
            0x9000..=0x97ff => {
                let offset = offset - 0x8800;
                self.tile_block_2[(offset / 16) as usize].lines[(offset % 16) as usize] = byte;
            }
            0x9800..=0x9bff => {
                let offset = offset - 0x9800;
                self.background_map_0[offset as usize] = byte;
            }
            0x9c00..=0x9fff => {
                let offset = offset - 0x9c00;
                self.background_map_1[offset as usize] = byte;
            }
            _ => unreachable!(),
        }
    }

    pub fn read_tile(&self, addressing_method: TileAddressingMethod) -> Tile {
        match addressing_method {
            TileAddressingMethod::From8000(offset) => {
                if offset > 127 {
                    self.tile_block_1[(offset - 128) as usize]
                } else {
                    self.tile_block_0[offset as usize]
                }
            }
            TileAddressingMethod::From9000(offset) => {
                if offset < 0 {
                    self.tile_block_1[(128i16 + offset as i16) as usize]
                } else {
                    self.tile_block_2[offset as usize]
                }
            }
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SpritePaletteNumber {
    Palette0,
    Palette1,
}

impl Default for SpritePaletteNumber {
    fn default() -> Self {
        Self::Palette0
    }
}

impl SpritePaletteNumber {
    fn new(v: u8) -> Self {
        match v {
            0 => Self::Palette0,
            1 => Self::Palette1,
            _ => unreachable!(),
        }
    }
}

impl From<SpritePaletteNumber> for u8 {
    fn from(bank: SpritePaletteNumber) -> Self {
        match bank {
            SpritePaletteNumber::Palette0 => 0,
            SpritePaletteNumber::Palette1 => 1,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SpriteVideoRamBank {
    Bank0,
    Bank1,
}

impl Default for SpriteVideoRamBank {
    fn default() -> Self {
        Self::Bank0
    }
}

impl SpriteVideoRamBank {
    fn new(v: u8) -> Self {
        match v {
            0 => Self::Bank0,
            1 => Self::Bank1,
            _ => unreachable!(),
        }
    }
}

impl From<SpriteVideoRamBank> for u8 {
    fn from(bank: SpriteVideoRamBank) -> Self {
        match bank {
            SpriteVideoRamBank::Bank0 => 0,
            SpriteVideoRamBank::Bank1 => 1,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct SpriteAttributes {
    behind_background: bool,
    flip_y: bool,
    flip_x: bool,
    gb_palette_number: SpritePaletteNumber,
    cgb_vram_bank: SpriteVideoRamBank,
    cgb_palette_number: u8,
}

impl From<u8> for SpriteAttributes {
    fn from(v: u8) -> Self {
        Self {
            behind_background: (v >> 7) != 0,
            flip_y: ((v >> 6) & 1) != 0,
            flip_x: ((v >> 5) & 1) != 0,
            gb_palette_number: SpritePaletteNumber::new((v >> 4) & 1),
            cgb_vram_bank: SpriteVideoRamBank::new((v >> 3) & 1),
            cgb_palette_number: v & 3,
        }
    }
}

impl From<SpriteAttributes> for u8 {
    fn from(attributes: SpriteAttributes) -> Self {
        ((attributes.behind_background as u8) << 7u8)
            | ((attributes.flip_y as u8) << 6u8)
            | ((attributes.flip_x as u8) << 5u8)
            | (u8::from(attributes.gb_palette_number) << 4u8)
            | (u8::from(attributes.cgb_vram_bank) << 3u8)
            | attributes.cgb_palette_number
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Sprite {
    y: u8,
    x: u8,
    tile_number: u8,
    attributes: SpriteAttributes,
}

impl Sprite {
    fn read(&self, offset: u16) -> u8 {
        match offset {
            0 => self.y,
            1 => self.x,
            2 => self.tile_number,
            3 => self.attributes.into(),
            _ => unreachable!()
        }
    }

    fn write(&mut self, offset: u16, byte: u8) {
        match offset {
            0 => self.y = byte,
            1 => self.x = byte,
            2 => self.tile_number = byte,
            3 => self.attributes = byte.into(),
            _ => unreachable!()
        }
    }
}

#[derive(Debug)]
pub struct ObjectAttributeMemory {
    sprites: [Sprite; 40],
}

impl Default for ObjectAttributeMemory {
    fn default() -> Self {
        Self {
            sprites: [Sprite::default(); 40],
        }
    }
}

impl ObjectAttributeMemory {
    fn read(&self, offset: u16) -> u8 {
        // Sprites are 4 bytes
        self.sprites[(offset / 4) as usize].read(offset % 4)
    }

    fn write(&mut self, offset: u16, byte: u8) {
        // Sprites are 4 bytes
        self.sprites[(offset / 4) as usize].write(offset % 4, byte);
    }
}

#[derive(Debug, Default)]
pub struct PictureProcessingUnit {
    in_use_by_lcd: bool,
    video_ram: VideoRam,
    object_attribute_memory: ObjectAttributeMemory,
}

impl PictureProcessingUnit {
    pub fn read_video_ram(&self, offset: u16) -> u8 {
        if self.in_use_by_lcd {
            return 0xff;
        }
        self.video_ram.read(offset)
    }

    pub fn write_video_ram(&mut self, offset: u16, byte: u8) {
        if !self.in_use_by_lcd {
            self.video_ram.write(offset, byte);
        }
    }

    pub fn read_object_attribute_memory(&self, offset: u16) -> u8 {
        if self.in_use_by_lcd {
            return 0xff;
        }
        self.object_attribute_memory.read(offset)
    }

    pub fn write_object_attribute_memory(&mut self, offset: u16, byte: u8) {
        if !self.in_use_by_lcd {
            self.object_attribute_memory.write(offset, byte);
        }
    }

    pub fn set_in_use_by_lcd(&mut self, in_use: bool) {
        self.in_use_by_lcd = in_use;
    }
}
