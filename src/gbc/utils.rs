
#[derive(Clone, Copy, Debug)]
pub enum Flag {
    On,
    Off,
}

impl Default for Flag {
    fn default() -> Self {
        Self::Off
    }
}

impl From<bool> for Flag {
    fn from(b: bool) -> Self {
        if b {
            Self::On
        } else {
            Self::Off
        }
    }
}

impl Flag {
    #[must_use]
    pub fn to_bool(self) -> bool {
        matches!(self, Self::On)
    }
}