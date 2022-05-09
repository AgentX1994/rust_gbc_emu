use crate::gbc::InputState;

#[derive(Debug)]
pub struct Joypad {
    input: u8, // (action << 4) | directions, a 0 means pressed
    selected: u8,
}

impl Default for Joypad {
    fn default() -> Self {
        Self {
            input: 0xff,
            selected: 0,
        }
    }
}

impl Joypad {
    pub fn write_u8(&mut self, byte: u8) {
        let byte = byte & 0x30; // mask out only the select bits;
        self.selected = byte;
    }

    #[must_use]
    pub fn read_u8(&self) -> u8 {
        let key_selection = (self.selected & 0b00110000) >> 4;
        match key_selection {
            0 | 3 => 0b11000000 | self.selected | 0b1111, // No selection or both selected
            1 => 0b11000000 | self.selected | (self.input & 0b1111), // Action buttons
            2 => 0b11000000 | self.selected | ((self.input >> 4) & 0b1111), // direction buttons
            _ => unreachable!()
        }
    }

    pub fn set_input_state(&mut self, input_state: &InputState) {
        let mut joypad_state = 0u8;
        if !input_state.a_pressed {
            joypad_state |= 0x1;
        }
        if !input_state.b_pressed {
            joypad_state |= 0x2;
        }
        if !input_state.select_pressed {
            joypad_state |= 0x4;
        }
        if !input_state.start_pressed {
            joypad_state |= 0x8;
        }
        if !input_state.right_pressed {
            joypad_state |= 0x10;
        }
        if !input_state.left_pressed {
            joypad_state |= 0x20;
        }
        if !input_state.up_pressed {
            joypad_state |= 0x40;
        }
        if !input_state.down_pressed {
            joypad_state |= 0x80;
        }
        self.input = joypad_state
    }
}
