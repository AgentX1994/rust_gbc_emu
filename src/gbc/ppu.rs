use std::collections::BinaryHeap;

use super::mmio::lcd::{Color, Lcd, SpriteSize, TileMap};

#[derive(Copy, Clone, Debug)]
pub enum ColorIndex {
    Color0,
    Color1,
    Color2,
    Color3,
}

impl ColorIndex {
    #[must_use]
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

impl Default for ColorIndex {
    fn default() -> Self {
        Self::Color0
    }
}

#[derive(Copy, Clone, Debug, Default)]
pub struct Tile {
    lines: [u8; 16],
}

impl Tile {
    #[must_use]
    fn get_color(&self, x: u8, y: u8) -> ColorIndex {
        assert!(x < 8);
        assert!(y < 8);

        let bit_index = 7 - x;
        let index = 2 * y;
        let byte1 = self.lines[index as usize];
        let byte2 = self.lines[(index + 1) as usize];
        let bit1 = (byte1 >> bit_index) & 1;
        let bit2 = (byte2 >> bit_index) & 1;
        ColorIndex::new((bit2 << 1) | bit1)
    }

    #[must_use]
    pub fn deinterleave(&self) -> [ColorIndex; 64] {
        let mut colors = [ColorIndex::default(); 64];
        for i in (0..16).step_by(2) {
            let mut byte0 = self.lines[i];
            let mut byte1 = self.lines[i + 1];

            colors[i * 8] = ColorIndex::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 1] = ColorIndex::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 2] = ColorIndex::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 3] = ColorIndex::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 4] = ColorIndex::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 5] = ColorIndex::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 6] = ColorIndex::new(((byte1 & 1) << 1) | (byte0 & 1));
            byte0 >>= 1;
            byte1 >>= 1;

            colors[i * 8 + 7] = ColorIndex::new(((byte1 & 1) << 1) | (byte0 & 1));
        }
        colors
    }
}

#[derive(Copy, Clone, Debug)]
pub enum TileAddressingMethod {
    From8000(u8),
    From9000(i8),
}

impl TileAddressingMethod {
    fn set_offset(&mut self, offset: u8) {
        match self {
            TileAddressingMethod::From8000(ref mut o) => *o = offset,
            #[allow(clippy::cast_possible_wrap)]
            TileAddressingMethod::From9000(ref mut i) => *i = offset as i8,
        }
    }
}

impl From<bool> for TileAddressingMethod {
    fn from(v: bool) -> Self {
        if v {
            Self::From8000(0)
        } else {
            Self::From9000(0)
        }
    }
}

impl From<TileAddressingMethod> for bool {
    fn from(s: TileAddressingMethod) -> Self {
        match s {
            TileAddressingMethod::From8000(_) => false,
            TileAddressingMethod::From9000(_) => true,
        }
    }
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
            0..=0x07ff => self.tile_block_0[(offset / 16) as usize].lines[(offset % 16) as usize],
            0x0800..=0x0fff => {
                let offset = offset - 0x0800;
                self.tile_block_1[(offset / 16) as usize].lines[(offset % 16) as usize]
            }
            0x1000..=0x17ff => {
                let offset = offset - 0x1000;
                self.tile_block_2[(offset / 16) as usize].lines[(offset % 16) as usize]
            }
            0x1800..=0x1bff => {
                let offset = offset - 0x1800;
                self.background_map_0[offset as usize]
            }
            0x1c00..=0x1fff => {
                let offset = offset - 0x1c00;
                self.background_map_1[offset as usize]
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, offset: u16, byte: u8) {
        match offset {
            0..=0x07ff => {
                self.tile_block_0[(offset / 16) as usize].lines[(offset % 16) as usize] = byte;
            }
            0x0800..=0x0fff => {
                let offset = offset - 0x0800;
                self.tile_block_1[(offset / 16) as usize].lines[(offset % 16) as usize] = byte;
            }
            0x1000..=0x17ff => {
                let offset = offset - 0x1000;
                self.tile_block_2[(offset / 16) as usize].lines[(offset % 16) as usize] = byte;
            }
            0x1800..=0x1bff => {
                let offset = offset - 0x1800;
                self.background_map_0[offset as usize] = byte;
            }
            0x1c00..=0x1fff => {
                let offset = offset - 0x1c00;
                self.background_map_1[offset as usize] = byte;
            }
            _ => unreachable!(),
        }
    }

    #[must_use]
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
                    #[allow(clippy::cast_sign_loss)]
                    self.tile_block_1[(128_i16 + i16::from(offset)) as usize]
                } else {
                    #[allow(clippy::cast_sign_loss)]
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
        ((attributes.behind_background as u8) << 7_u8)
            | ((attributes.flip_y as u8) << 6_u8)
            | ((attributes.flip_x as u8) << 5_u8)
            | (u8::from(attributes.gb_palette_number) << 4_u8)
            | (u8::from(attributes.cgb_vram_bank) << 3_u8)
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
            _ => unreachable!(),
        }
    }

    fn write(&mut self, offset: u16, byte: u8) {
        match offset {
            0 => self.y = byte,
            1 => self.x = byte,
            2 => self.tile_number = byte,
            3 => self.attributes = byte.into(),
            _ => unreachable!(),
        }
    }
}

impl PartialEq for Sprite {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x
    }
}

impl PartialOrd for Sprite {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.x.partial_cmp(&self.x)
    }
}

impl Eq for Sprite {}

impl Ord for Sprite {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.x.cmp(&self.x)
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

#[derive(Debug)]
pub struct PictureProcessingUnit {
    in_use_by_lcd: bool,
    video_ram: VideoRam,
    object_attribute_memory: ObjectAttributeMemory,
    framebuffer1: [[Color; 160]; 144],
    framebuffer2: [[Color; 160]; 144],
    framebuffer_selector: bool,
    sprites_this_line: BinaryHeap<Sprite>,
}

impl Default for PictureProcessingUnit {
    fn default() -> Self {
        Self {
            in_use_by_lcd: false,
            video_ram: VideoRam::default(),
            object_attribute_memory: ObjectAttributeMemory::default(),
            framebuffer1: [[Color::White; 160]; 144],
            framebuffer2: [[Color::White; 160]; 144],
            framebuffer_selector: false,
            sprites_this_line: BinaryHeap::new(),
        }
    }
}

impl PictureProcessingUnit {
    #[must_use]
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

    #[must_use]
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

    fn get_color_at_pixel_using_tilemap(
        &self,
        x: u8,
        y: u8,
        selected_map: TileMap,
        mut addressing_mode: TileAddressingMethod,
    ) -> ColorIndex {
        let tilemap = match selected_map {
            TileMap::From9800 => &self.video_ram.background_map_0,
            TileMap::From9C00 => &self.video_ram.background_map_1,
        };
        let map_x = x / 8;
        let map_y = y / 8;
        let tile_index = tilemap[32 * map_y as usize + map_x as usize];

        addressing_mode.set_offset(tile_index);
        let tile = self.video_ram.read_tile(addressing_mode);

        let tile_x = x % 8;
        let tile_y = y % 8;
        tile.get_color(tile_x, tile_y)
    }

    fn get_color_at_pixel_for_sprite(
        &self,
        x: u8,
        y: u8,
        tile_index: u8,
        flip_x: bool,
        flip_y: bool,
    ) -> ColorIndex {
        // Sprites only use tilemap 1
        let addressing_mode = TileAddressingMethod::From8000(tile_index);
        let tile = self.video_ram.read_tile(addressing_mode);
        let tile_x = if flip_x { 7 - (x % 8) } else { x % 8 };
        let tile_y = if flip_y { 7 - (y % 8) } else { y % 8 };
        tile.get_color(tile_x, tile_y)
    }

    fn get_sprites_for_line(&mut self, y: u8, sprite_size: SpriteSize) {
        self.sprites_this_line.clear();
        for ref object in self.object_attribute_memory.sprites {
            let sprite_y = i16::from(object.y) - 16;
            let y_i16 = i16::from(y);

            let should_be_drawn = match sprite_size {
                SpriteSize::Small => (sprite_y..(sprite_y + 8)).contains(&y_i16),
                SpriteSize::Large => (sprite_y..(sprite_y + 16)).contains(&y_i16),
            };

            if should_be_drawn {
                self.sprites_this_line.push(*object);

                if self.sprites_this_line.len() >= 10 {
                    break;
                }
            }
        }
    }

    pub fn tick(&mut self, cycles: u64, lcd: &mut Lcd) -> (bool, bool) {
        let mut vblank_interrupt = false;
        let mut stat_interrupt = false;
        let lcd_enable = lcd.get_lcd_enable();

        if lcd_enable {
            for _ in 0..cycles {
                let x_i16 = lcd.get_lx();
                let y = lcd.get_ly();
                if x_i16 == -80 {
                    // New line, reset sprites drawn so far and get the sprites for this line
                    self.get_sprites_for_line(y, lcd.get_sprite_size());
                }

                if (0..160).contains(&x_i16) && y < 144 {
                    #[allow(clippy::cast_possible_truncation)]
                    let x = x_i16 as u8;
                    let addressing_mode = lcd.get_addressing_mode();
                    let bg_window_priority = lcd.get_background_window_priority();

                    // draw background
                    let (scroll_x, scroll_y) = lcd.get_scroll_offsets();
                    let bg_x = scroll_x.wrapping_add(x);
                    let bg_y = scroll_y.wrapping_add(y);
                    let bg_tile_map = lcd.get_background_tile_map();
                    let palette = lcd.get_background_palette();

                    // If this is not true, the background and window should display as white
                    let mut bg_color_index_was_zero;
                    if bg_window_priority {
                        let bg_color = self.get_color_at_pixel_using_tilemap(
                            bg_x,
                            bg_y,
                            bg_tile_map,
                            addressing_mode,
                        );
                        bg_color_index_was_zero = matches!(bg_color, ColorIndex::Color0);
                        let color = palette.get_color(&bg_color);
                        self.write_to_framebuffer(x as usize, y as usize, color);
                    } else {
                        bg_color_index_was_zero = true;
                        self.write_to_framebuffer(x as usize, y as usize, Color::White);
                    }

                    // draw window
                    let (mut window_x, window_y) = lcd.get_window_coords();
                    window_x = window_x.saturating_sub(7);
                    let window_tile_map = lcd.get_window_tile_map();
                    let window_enable = lcd.get_window_enable();

                    if bg_window_priority
                        && window_enable
                        && x >= window_x
                        && y >= window_y
                    {
                        let win_pos_x = x - window_x;
                        let win_pos_y = y - window_y;
                        let bg_color = self.get_color_at_pixel_using_tilemap(
                            win_pos_x,
                            win_pos_y,
                            window_tile_map,
                            addressing_mode,
                        );
                        let color = palette.get_color(&bg_color);
                        self.write_to_framebuffer(x as usize, y as usize, color);
                        bg_color_index_was_zero = matches!(bg_color, ColorIndex::Color0);
                    }

                    // draw objects
                    let objects_enable = lcd.get_sprite_enable();
                    let sprite_size = lcd.get_sprite_size();
                    if objects_enable {
                        // Loop over each object and calculate if it should be drawn
                        for object in self.sprites_this_line.iter() {
                            let sprite_y = i16::from(object.y) - 16;
                            let sprite_x = i16::from(object.x) - 8;
                            let y_i16 = i16::from(y);

                            let mut should_be_drawn = match sprite_size {
                                SpriteSize::Small => (sprite_x..(sprite_x + 8)).contains(&x_i16),
                                SpriteSize::Large => (sprite_x..(sprite_x + 8)).contains(&x_i16),
                            };
                            should_be_drawn &=
                                !object.attributes.behind_background || bg_color_index_was_zero;

                            if should_be_drawn {
                                let color_index = match sprite_size {
                                    SpriteSize::Small => {
                                        let tile_x = (x_i16 - sprite_x) as u8;
                                        let tile_y = (y_i16 - sprite_y) as u8;
                                        self.get_color_at_pixel_for_sprite(
                                            tile_x,
                                            tile_y,
                                            object.tile_number,
                                            object.attributes.flip_x,
                                            object.attributes.flip_y,
                                        )
                                    }
                                    SpriteSize::Large => {
                                        let tile_x = (x_i16 - sprite_x) as u8;
                                        let tile_y = (y_i16 - sprite_y) as u8;
                                        // The hardware enforces that, for two tile
                                        // sprites, the first sprite has a 0 in the lowest
                                        // bit, and the second sprite has a 1
                                        let temp_tile_index = if tile_y > 7 {
                                            object.tile_number | 0x1
                                        } else {
                                            object.tile_number & !0x1
                                        };
                                        let tile_index = if object.attributes.flip_y {
                                            temp_tile_index ^ 0x1
                                        } else {
                                            temp_tile_index
                                        };
                                        self.get_color_at_pixel_for_sprite(
                                            tile_x,
                                            tile_y,
                                            tile_index,
                                            object.attributes.flip_x,
                                            object.attributes.flip_y,
                                        )
                                    }
                                };

                                if !matches!(color_index, ColorIndex::Color0) {
                                    let (obj_pal0, obj_pal1) = lcd.get_object_palettes();
                                    let palette = match object.attributes.gb_palette_number {
                                        SpritePaletteNumber::Palette0 => obj_pal0,
                                        SpritePaletteNumber::Palette1 => obj_pal1,
                                    };
                                    let color = palette.get_color(&color_index);
                                    self.write_to_framebuffer(x as usize, y as usize, color);
                                    break;
                                }
                            }
                        }
                    }
                }

                let interrupts = lcd.tick();
                if interrupts.0 {
                    self.framebuffer_selector ^= true;
                }
                vblank_interrupt |= interrupts.0;
                stat_interrupt |= interrupts.1;
            }
        }

        (vblank_interrupt, stat_interrupt)
    }

    fn write_to_framebuffer(&mut self, x: usize, y: usize, color: Color) {
        let framebuffer = self.get_current_framebuffer_mut();
        framebuffer[y][x] = color;
    }

    pub fn get_current_framebuffer_mut(&mut self) -> &mut [[Color; 160]; 144] {
        if self.framebuffer_selector {
            &mut self.framebuffer2
        } else {
            &mut self.framebuffer1
        }
    }

    #[must_use]
    pub fn get_current_framebuffer(&self) -> &[[Color; 160]; 144] {
        if self.framebuffer_selector {
            &self.framebuffer2
        } else {
            &self.framebuffer1
        }
    }
}
