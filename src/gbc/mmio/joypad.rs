
#[derive(Debug)]
pub struct Joypad {
    input: u8, // (action << 4) | directions, a 0 means pressed
    selected: u8
}

impl Default for Joypad {
    fn default() -> Self {
        Self {input: 0xff, 
            selected: 0 }
    }
}

impl Joypad {
    pub fn write_u8(&mut self, byte: u8) {
        let byte = byte & 0x30; // mask out only the select bits;
        self.selected = byte;
    }

    #[must_use]
    pub fn read_u8(&self) -> u8 {
        // I'm pretty sure this clippy lint is completely
        // wrong here
        #[allow(clippy::if_not_else)]
        if self.selected & (1 << 4) != 0 {
            0xc0 | self.selected | (self.input & 0xf)
        } else if self.selected & (1 << 5) != 0 {
            0xc0 | self.selected | ((self.input >> 4) & 0xf)
        } else {
            0xc0 | self.selected | 0xf
        }
    }
}
