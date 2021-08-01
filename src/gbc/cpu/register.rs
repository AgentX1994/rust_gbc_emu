use std::{
    fmt::{self, Debug, Display},
    u16,
};

#[derive(Clone, Copy)]
struct Inner {
    high: u8,
    low: u8,
}

#[derive(Clone, Copy)]
#[repr(C)]
union RegisterInner {
    all: u16,
    inner: Inner,
}

pub struct RegisterStorage {
    value: RegisterInner,
}

impl RegisterStorage {
    pub fn new(value: u16) -> Self {
        RegisterStorage {
            value: RegisterInner { all: value },
        }
    }

    pub fn set_low(&mut self, value: u8) {
        self.value.inner.low = value;
    }

    pub fn set_high(&mut self, value: u8) {
        self.value.inner.high = value;
    }

    pub fn set_u16(&mut self, value: u16) {
        self.value.all = value;
    }

    pub fn get_low(&self) -> u8 {
        unsafe { self.value.inner.low }
    }

    pub fn get_high(&self) -> u8 {
        unsafe { self.value.inner.high }
    }

    pub fn get_u16(&self) -> u16 {
        unsafe { self.value.all }
    }

    pub fn get_low_mut(&mut self) -> &mut u8 {
        unsafe { &mut self.value.inner.low }
    }

    pub fn get_high_mut(&mut self) -> &mut u8 {
        unsafe { &mut self.value.inner.high }
    }

    pub fn get_u16_mut(&mut self) -> &mut u16 {
        unsafe { &mut self.value.all }
    }
}

impl Default for RegisterStorage {
    fn default() -> Self {
        RegisterStorage {
            value: RegisterInner { all: 0 },
        }
    }
}

impl Debug for RegisterStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: always safe
        unsafe {
            f.debug_struct("Register")
                .field("value.all", &self.value.all)
                .finish()
        }
    }
}

impl Display for RegisterStorage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Safety: always safe
        unsafe { write!(f, "{:04x}", self.value.all) }
    }
}
