#[derive(Debug, Default)]
pub struct SerialComms {
    pub io_register: u8,
    pub control: u8,
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

#[derive(Debug, Default)]
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

#[derive(Debug, Default)]
pub struct Mmio {
    pub joypad_input: u8,
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

impl Mmio {
    pub fn read_u8(&self, address: u16) -> u8 {
        0
    }

    pub fn write_u8(&mut self, address: u16, byte: u8) {
        return;
    }
}
