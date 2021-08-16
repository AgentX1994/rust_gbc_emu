use std::fmt::{self, Display};

#[derive(Copy, Clone, Debug)]
pub enum AccessType {
    Read,
    Write,
    Execute,
    ReadWrite,
    ReadExecute,
    WriteExecute,
    ReadWriteExecute,
}

impl Display for AccessType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read => write!(f, "r__"),
            Self::Write => write!(f, "_w_"),
            Self::Execute => write!(f, "__e"),
            Self::ReadWrite => write!(f, "rw_"),
            Self::ReadExecute => write!(f, "r_e"),
            Self::WriteExecute => write!(f, "_we"),
            Self::ReadWriteExecute => write!(f, "rwe"),
        }
    }
}

impl AccessType {
    pub fn from_string(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "r" => Ok(Self::Read),
            "w" => Ok(Self::Write),
            "e" => Ok(Self::Execute),
            "rw" => Ok(Self::ReadWrite),
            "re" => Ok(Self::ReadExecute),
            "we" => Ok(Self::WriteExecute),
            "rwe" => Ok(Self::ReadWriteExecute),
            _ => Err(format!("Unknown Access Type {}", s)),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum BreakReason {
    User,
}

impl Display for BreakReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::User => write!(f, "USER"),
        }
    }
}

impl AccessType {
    #[must_use]
    pub fn on_read(&self) -> bool {
        matches!(self, Self::Read | Self::ReadWrite | Self::ReadExecute | Self::ReadWriteExecute)
    }

    #[must_use]
    pub fn on_write(&self) -> bool {
        matches!(self, Self::Write | Self::ReadWrite | Self::WriteExecute | Self::ReadWriteExecute)
    }

    #[must_use]
    pub fn on_execute(&self) -> bool {
        matches!(self, Self::Execute | Self::ReadExecute | Self::WriteExecute | Self::ReadWriteExecute)
    }
}

#[derive(Clone, Debug)]
pub struct Breakpoint {
    pub address: u16,
    pub access_type: AccessType,
    pub length: u16,
    pub reason: BreakReason,
}

impl Breakpoint {
    #[must_use]
    pub fn new(address: u16, access_type: AccessType, length: u16, reason: BreakReason) -> Self {
        Breakpoint {
            address,
            access_type,
            length,
            reason,
        }
    }

    #[must_use]
    pub fn matches_address(&self, address: u16) -> bool {
        self.address <= address && address < (self.address + self.length)
    }
}
