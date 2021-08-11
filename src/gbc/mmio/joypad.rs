
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

    pub fn read_u8(&self) -> u8 {
        if self.selected & (1 << 4) != 0 {
            0xd | self.selected | (self.input & 0xf)
        } else if self.selected & (1 << 5) != 0 {
            0xd | self.selected | ((self.input >> 4) & 0xf)
        } else {
            0xd | self.selected
        }
    }
}
