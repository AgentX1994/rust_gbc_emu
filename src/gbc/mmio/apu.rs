#[derive(Debug, Default)]
pub struct SoundChannel1 {
    pub sweep_control: u8,     // NR10
    pub sound_length_duty: u8, // NR11
    pub volume_envelope: u8,   // NR12
    pub frequency_low: u8,     // NR13
    pub frequency_high: u8,    // NR14
}

#[derive(Debug, Default)]
pub struct SoundChannel2 {
    pub sound_length_duty: u8, // NR21
    pub volume_envelope: u8,   // NR22
    pub frequency_low: u8,     // NR23
    pub frequency_high: u8,    // NR24
}

#[derive(Debug, Default)]
pub struct SoundDigitalChannel {
    pub is_on: bool,        // NR30
    pub length: u8,         // NR31
    pub volume: u8,         // NR32
    pub frequency_low: u8,  // NR33
    pub frequency_high: u8, // NR34
    pub wave_ram: [u8; 16],
}

#[derive(Debug, Default)]
pub struct SoundNoiseChannel {
    pub length: u8,             // NR41
    pub volume: u8,             // NR42
    pub polynomial_counter: u8, // NR43
    pub counter: u8,            // NR44
}

#[derive(Debug, Default)]
pub struct Sound {
    pub channel1: SoundChannel1,
    pub channel2: SoundChannel2,
    pub digital_channel: SoundDigitalChannel,
    pub noise_channel: SoundNoiseChannel,
    pub channel_control: u8,      // NR50
    pub sound_output_control: u8, // NR51
    pub sound_on_off_control: u8, // NR52
}

impl Sound {
    pub fn read_u8(&self, offset: u16) -> u8 {
        match offset {
            0x0 => self.channel1.sweep_control,
            0x1 => self.channel1.sound_length_duty,
            0x2 => self.channel1.volume_envelope,
            0x3 => 0, // write only 
            0x4 => self.channel1.frequency_high,
            0x5 => 0, // skip 5?
            0x6 => self.channel2.sound_length_duty,
            0x7 => self.channel2.volume_envelope,
            0x8 => 0, // write only 
            0x9 => self.channel2.frequency_high,
            0xa => self.digital_channel.is_on as u8,
            0xb => self.digital_channel.length,
            0xc => self.digital_channel.volume,
            0xd => self.digital_channel.frequency_low,
            0xe => self.digital_channel.frequency_high,
            // the wave ram is a seperate memory region
            0xf => 0, // 0xf is unused
            0x10 => self.noise_channel.length,
            0x11 => self.noise_channel.volume,
            0x12 => self.noise_channel.polynomial_counter,
            0x13 => self.noise_channel.counter,
            0x14 => self.channel_control,
            0x15 => self.sound_output_control,
            0x16 => self.sound_on_off_control,
            _ => unreachable!(),
        }
    }

    pub fn write_u8(&mut self, offset: u16, byte: u8) {
        match offset {
            0x0 => self.channel1.sweep_control = byte,
            0x1 => self.channel1.sound_length_duty = byte,
            0x2 => self.channel1.volume_envelope = byte,
            0x3 => self.channel1.frequency_low = byte,
            0x4 => self.channel1.frequency_high = byte,
            0x5 => (),// skip 5?
            0x6 => self.channel2.sound_length_duty = byte,
            0x7 => self.channel2.volume_envelope = byte,
            0x8 => self.channel2.frequency_low = byte,
            0x9 => self.channel2.frequency_high = byte,
            0xa => self.digital_channel.is_on = byte != 0,
            0xb => self.digital_channel.length = byte,
            0xc => self.digital_channel.volume = byte,
            0xd => self.digital_channel.frequency_low = byte,
            0xe => self.digital_channel.frequency_high = byte,
            // the wave ram is a seperate memory region
            0xf => (), // 0xf is unused
            0x10 => self.noise_channel.length = byte,
            0x11 => self.noise_channel.volume = byte,
            0x12 => self.noise_channel.polynomial_counter = byte,
            0x13 => self.noise_channel.counter = byte,
            0x14 => self.channel_control = byte,
            0x15 => self.sound_output_control = byte,
            0x16 => self.sound_on_off_control = byte,
            _ => unreachable!(),
        }
    }

    pub fn read_u8_from_waveform(&self, offset: u16) -> u8 {
        assert!(offset < 16);
        self.digital_channel.wave_ram[offset as usize]
    }

    pub fn write_u8_from_waveform(&mut self, offset: u16, byte: u8) {
        assert!(offset < 16);
        // TODO emulate weird behavior when CH3 is enabled
        self.digital_channel.wave_ram[offset as usize] = byte;
    }
}
