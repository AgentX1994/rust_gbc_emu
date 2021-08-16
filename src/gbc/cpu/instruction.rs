use core::fmt;
use std::{fmt::Display, mem};

use crate::gbc::memory_bus::MemoryBus;

#[derive(Debug, PartialEq)]
pub enum Register {
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    Bc,
    De,
    Hl,
    Sp,
    HlPlus,
    HlMinus,
    Af,
}

impl Display for Register {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Register::A => write!(f, "A"),
            Register::B => write!(f, "B"),
            Register::C => write!(f, "C"),
            Register::D => write!(f, "D"),
            Register::E => write!(f, "E"),
            Register::H => write!(f, "H"),
            Register::L => write!(f, "L"),
            Register::Bc => write!(f, "BC"),
            Register::De => write!(f, "DE"),
            Register::Hl => write!(f, "HL"),
            Register::Sp => write!(f, "SP"),
            Register::HlPlus => write!(f, "HL+"),
            Register::HlMinus => write!(f, "HL-"),
            Register::Af => write!(f, "AF"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DerefOperand {
    Register(Register),
    Address(u16),
    Ff00Offset(u8),
    Ff00PlusC,
}

impl Display for DerefOperand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DerefOperand::Register(r) => write!(f, "{}", r),
            DerefOperand::Address(addr) => write!(f, "{:#x}", addr),
            DerefOperand::Ff00Offset(offset) => write!(f, "0xff00+{:#x}", offset),
            DerefOperand::Ff00PlusC => write!(f, "0xff00+C"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Operand {
    Register(Register),
    I8(i8),
    U8(u8),
    U16(u16),
    Deref(DerefOperand),
    StackOffset(i8),
}

impl Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::U16(x) => write!(f, "{:#x}", x),
            Operand::U8(x) => write!(f, "{:#x}", x),
            Operand::I8(x) => write!(f, "{}", x),
            Operand::Register(r) => write!(f, "{}", r),
            Operand::Deref(operand) => write!(f, "({})", operand),
            Operand::StackOffset(offset) => {
                if *offset > 0 {
                    write!(f, "sp+{:#x}", offset)
                } else {
                    write!(f, "sp-{:#x}", -offset)
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum ConditionType {
    NonZero,
    Zero,
    NotCarry,
    Carry,
}

impl Display for ConditionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConditionType::Zero => write!(f, "z"),
            ConditionType::NonZero => write!(f, "nz"),
            ConditionType::Carry => write!(f, "c"),
            ConditionType::NotCarry => write!(f, "nc"),
        }
    }
}

#[derive(Debug)]
pub enum Opcode {
    Unknown {
        opcode: u8,
    },
    Nop,
    Stop,
    Halt,
    Ld8 {
        destination: Operand,
        source: Operand,
    },
    Ld16 {
        destination: Operand,
        source: Operand,
    },
    Jp {
        destination: Operand,
    },
    JpCond {
        condition: ConditionType,
        destination: u16,
    },
    Jr {
        offset: i8,
    },
    JrCond {
        condition: ConditionType,
        offset: i8,
    },
    Call {
        destination: u16,
    },
    CallCond {
        condition: ConditionType,
        destination: u16,
    },
    Ret,
    RetCond {
        condition: ConditionType,
    },
    Reti,
    Pop {
        register: Register,
    },
    Push {
        register: Register,
    },
    Rst {
        vector: u8,
    },
    Bit {
        bit: u8,
        destination: Operand,
    },
    Res {
        bit: u8,
        destination: Operand,
    },
    Set {
        bit: u8,
        destination: Operand,
    },
    Add8 {
        operand: Operand,
    },
    Add16 {
        register: Register,
        operand: Operand,
    },
    Inc {
        operand: Operand,
    },
    Inc16 {
        register: Register,
    },
    Dec {
        operand: Operand,
    },
    Dec16 {
        register: Register,
    },
    Adc {
        operand: Operand,
    },
    Sub {
        operand: Operand,
    },
    Sbc {
        operand: Operand,
    },
    And {
        operand: Operand,
    },
    Xor {
        operand: Operand,
    },
    Or {
        operand: Operand,
    },
    Cp {
        operand: Operand,
    },
    Cpl,
    Daa,
    Rlca,
    Rla,
    Rrca,
    Rra,
    Rlc {
        operand: Operand,
    },
    Rl {
        operand: Operand,
    },
    Rrc {
        operand: Operand,
    },
    Rr {
        operand: Operand,
    },
    Sla {
        operand: Operand,
    },
    Swap {
        operand: Operand,
    },
    Sra {
        operand: Operand,
    },
    Srl {
        operand: Operand,
    },
    Scf,
    Ccf,
    Di,
    Ei,
}

impl Opcode {
    fn print(&self, address: u16) -> String {
        #[allow(clippy::match_same_arms)]
        match self {
            Opcode::Unknown { opcode } => format!("Unknown: {0:02x} {0:08b}", opcode),
            Opcode::Nop => "nop".to_string(),
            Opcode::Stop => "stop".to_string(),
            Opcode::Halt => "halt".to_string(),
            Opcode::Ld8 {
                destination,
                source,
            } => format!("ld {} {}", destination, source),
            Opcode::Ld16 {
                destination,
                source,
            } => format!("ld {} {}", destination, source),
            Opcode::Jp { destination } => format!("jp {}", destination),
            Opcode::JpCond {
                condition,
                destination,
            } => format!("jp {},{:#x}", condition, destination),
            Opcode::Jr { offset } => format!("jr {:#x}", i32::from(address) + i32::from(*offset)),
            Opcode::JrCond { condition, offset } => {
                format!(
                    "jr {},{:#x}",
                    condition,
                    i32::from(address) + i32::from(*offset)
                )
            }
            Opcode::Call { destination } => format!("call {:#x}", destination),
            Opcode::CallCond {
                condition,
                destination,
            } => format!("call {},{:#x}", condition, destination),
            Opcode::Ret => "ret".to_string(),
            Opcode::RetCond { condition } => format!("ret {}", condition),
            Opcode::Reti => "reti".to_string(),
            Opcode::Pop { register } => format!("pop {}", register),
            Opcode::Push { register } => format!("push {}", register),
            Opcode::Rst { vector } => format!("rst {:#x}", vector),
            Opcode::Bit { bit, destination } => format!("bit {},{}", bit, destination),
            Opcode::Res { bit, destination } => format!("res {},{}", bit, destination),
            Opcode::Set { bit, destination } => format!("set {},{}", bit, destination),
            Opcode::Add8 { operand } => format!("add A,{}", operand),
            Opcode::Add16 { register, operand } => format!("add {},{}", register, operand),
            Opcode::Inc { operand } => format!("inc {}", operand),
            Opcode::Inc16 { register } => format!("inc {}", register),
            Opcode::Dec { operand } => format!("dec {}", operand),
            Opcode::Dec16 { register } => format!("dec {}", register),
            Opcode::Adc { operand } => format!("adc A,{}", operand),
            Opcode::Sub { operand } => format!("sub {}", operand),
            Opcode::Sbc { operand } => format!("sbc A,{}", operand),
            Opcode::And { operand } => format!("and {}", operand),
            Opcode::Xor { operand } => format!("xor {}", operand),
            Opcode::Or { operand } => format!("or {}", operand),
            Opcode::Cp { operand } => format!("cp {}", operand),
            Opcode::Cpl => "cpl".to_string(),
            Opcode::Daa => "daa".to_string(),
            Opcode::Rlca => "rlca".to_string(),
            Opcode::Rla => "rla".to_string(),
            Opcode::Rrca => "rrca".to_string(),
            Opcode::Rra => "rra".to_string(),
            Opcode::Rlc { operand: register } => format!("rlc {}", register),
            Opcode::Rl { operand: register } => format!("rl {}", register),
            Opcode::Rrc { operand: register } => format!("rrc {}", register),
            Opcode::Rr { operand: register } => format!("rr {}", register),
            Opcode::Sla { operand: register } => format!("sla {}", register),
            Opcode::Swap { operand: register } => format!("swap {}", register),
            Opcode::Sra { operand: register } => format!("sra {}", register),
            Opcode::Srl { operand: register } => format!("Srl {}", register),
            Opcode::Scf => "scf".to_string(),
            Opcode::Ccf => "ccf".to_string(),
            Opcode::Di => "di".to_string(),
            Opcode::Ei => "ei".to_string(),
        }
    }

    fn size(&self) -> u8 {
        #[allow(clippy::match_same_arms)]
        match self {
            Opcode::Unknown { .. } => 1,
            Opcode::Nop => 1,
            Opcode::Stop => 2,
            Opcode::Halt => 1,
            Opcode::Ld8 {
                destination,
                source,
            } => {
                if let Operand::U8(..) = source {
                    2
                } else if let Operand::Deref(DerefOperand::Ff00Offset(..)) = source {
                    2
                } else if let Operand::Deref(DerefOperand::Address(..)) = source {
                    3
                } else if let Operand::Deref(DerefOperand::Address(..)) = destination {
                    3
                } else if let Operand::Deref(DerefOperand::Ff00Offset(..)) = destination {
                    2
                } else {
                    1
                }
            }
            Opcode::Ld16 {
                destination,
                source,
            } => {
                if let Operand::Deref(DerefOperand::Address(..)) = destination {
                    3
                } else if let Operand::U16(..) = source {
                    3
                } else if let Operand::StackOffset(..) = source {
                    2
                } else {
                    1
                }
            }
            Opcode::Jp { destination } => {
                if let Operand::Register(Register::Hl) = destination {
                    1
                } else {
                    3
                }
            }
            Opcode::JpCond { .. } => 3,
            Opcode::Jr { .. } => 2,
            Opcode::JrCond { .. } => 2,
            Opcode::Call { .. } => 3,
            Opcode::CallCond { .. } => 3,
            Opcode::Ret => 1,
            Opcode::RetCond { .. } => 1,
            Opcode::Reti => 1,
            Opcode::Pop { .. } => 1,
            Opcode::Push { .. } => 1,
            Opcode::Rst { .. } => 1,
            Opcode::Bit { .. } => 2,
            Opcode::Res { .. } => 2,
            Opcode::Set { .. } => 2,
            Opcode::Add8 { operand } => {
                if let Operand::U8(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::Add16 { operand, .. } => {
                if let Operand::I8(..) = operand {
                    2
                } else if let Operand::StackOffset(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::Inc { .. } => 1,
            Opcode::Inc16 { .. } => 1,
            Opcode::Dec { .. } => 1,
            Opcode::Dec16 { .. } => 1,
            Opcode::Adc { operand } => {
                if let Operand::U8(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::Sub { operand } => {
                if let Operand::U8(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::Sbc { operand } => {
                if let Operand::U8(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::And { operand } => {
                if let Operand::U8(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::Xor { operand } => {
                if let Operand::U8(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::Or { operand } => {
                if let Operand::U8(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::Cp { operand } => {
                if let Operand::U8(..) = operand {
                    2
                } else {
                    1
                }
            }
            Opcode::Cpl => 1,
            Opcode::Daa => 1,
            Opcode::Rlca => 1,
            Opcode::Rla => 1,
            Opcode::Rrca => 1,
            Opcode::Rra => 1,
            Opcode::Rlc { .. } => 2,
            Opcode::Rl { .. } => 2,
            Opcode::Rrc { .. } => 2,
            Opcode::Rr { .. } => 2,
            Opcode::Sla { .. } => 2,
            Opcode::Swap { .. } => 2,
            Opcode::Sra { .. } => 2,
            Opcode::Srl { .. } => 2,
            Opcode::Scf => 1,
            Opcode::Ccf => 1,
            Opcode::Di => 1,
            Opcode::Ei => 1,
        }
    }
}

fn make_u16(low: u8, high: u8) -> u16 {
    u16::from(high) << 8 | u16::from(low)
}

fn make_i8(v: u8) -> i8 {
    unsafe { mem::transmute(v) }
}

#[derive(Debug)]
pub struct Instruction {
    pub address: u16,
    pub op: Opcode,
}

impl Instruction {
    #[must_use]
    pub fn size(&self) -> u8 {
        self.op.size()
    }

    #[must_use]
    pub fn new(address: u16, memory_bus: &MemoryBus) -> Self {
        let byte = memory_bus.read_u8(address);

        #[allow(clippy::match_same_arms)]
        match byte {
            0x00 => Instruction {
                address,
                op: Opcode::Nop,
            },
            0x01 => Instruction {
                address,
                op: Opcode::Ld16 {
                    destination: Operand::Register(Register::Bc),
                    source: Operand::U16(make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    )),
                },
            },
            0x02 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Bc)),
                    source: Operand::Register(Register::A),
                },
            },
            0x03 => Instruction {
                address,
                op: Opcode::Inc16 {
                    register: Register::Bc,
                },
            },
            0x04 => Instruction {
                address,
                op: Opcode::Inc {
                    operand: Operand::Register(Register::B),
                },
            },
            0x05 => Instruction {
                address,
                op: Opcode::Dec {
                    operand: Operand::Register(Register::B),
                },
            },
            0x06 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x07 => Instruction {
                address,
                op: Opcode::Rlca,
            },
            0x08 => Instruction {
                address,
                op: Opcode::Ld16 {
                    destination: Operand::Deref(DerefOperand::Address(make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ))),
                    source: Operand::Register(Register::Sp),
                },
            },
            0x09 => Instruction {
                address,
                op: Opcode::Add16 {
                    register: Register::Hl,
                    operand: Operand::Register(Register::Bc),
                },
            },
            0x0a => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Register(Register::Bc)),
                },
            },
            0x0b => Instruction {
                address,
                op: Opcode::Dec16 {
                    register: Register::Bc,
                },
            },
            0x0c => Instruction {
                address,
                op: Opcode::Inc {
                    operand: Operand::Register(Register::C),
                },
            },
            0x0d => Instruction {
                address,
                op: Opcode::Dec {
                    operand: Operand::Register(Register::C),
                },
            },
            0x0e => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x0f => Instruction {
                address,
                op: Opcode::Rrca,
            },
            0x10 => {
                let next_byte = memory_bus.read_u8(address.wrapping_add(1));
                if next_byte != 0 {
                    println!("Corrupted STOP!");
                }
                Instruction {
                    address,
                    op: Opcode::Stop,
                }
            }
            0x11 => Instruction {
                address,
                op: Opcode::Ld16 {
                    destination: Operand::Register(Register::De),
                    source: Operand::U16(make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    )),
                },
            },
            0x12 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::De)),
                    source: Operand::Register(Register::A),
                },
            },
            0x13 => Instruction {
                address,
                op: Opcode::Inc16 {
                    register: Register::De,
                },
            },
            0x14 => Instruction {
                address,
                op: Opcode::Inc {
                    operand: Operand::Register(Register::D),
                },
            },
            0x15 => Instruction {
                address,
                op: Opcode::Dec {
                    operand: Operand::Register(Register::D),
                },
            },
            0x16 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x17 => Instruction {
                address,
                op: Opcode::Rla,
            },
            0x18 => Instruction {
                address,
                op: Opcode::Jr {
                    offset: make_i8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x19 => Instruction {
                address,
                op: Opcode::Add16 {
                    register: Register::Hl,
                    operand: Operand::Register(Register::De),
                },
            },
            0x1a => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Register(Register::De)),
                },
            },
            0x1b => Instruction {
                address,
                op: Opcode::Dec16 {
                    register: Register::De,
                },
            },
            0x1c => Instruction {
                address,
                op: Opcode::Inc {
                    operand: Operand::Register(Register::E),
                },
            },
            0x1d => Instruction {
                address,
                op: Opcode::Dec {
                    operand: Operand::Register(Register::E),
                },
            },
            0x1e => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x1f => Instruction {
                address,
                op: Opcode::Rra,
            },
            0x20 => Instruction {
                address,
                op: Opcode::JrCond {
                    condition: ConditionType::NonZero,
                    offset: make_i8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x21 => Instruction {
                address,
                op: Opcode::Ld16 {
                    destination: Operand::Register(Register::Hl),
                    source: Operand::U16(make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    )),
                },
            },
            0x22 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::HlPlus)),
                    source: Operand::Register(Register::A),
                },
            },
            0x23 => Instruction {
                address,
                op: Opcode::Inc16 {
                    register: Register::Hl,
                },
            },
            0x24 => Instruction {
                address,
                op: Opcode::Inc {
                    operand: Operand::Register(Register::H),
                },
            },
            0x25 => Instruction {
                address,
                op: Opcode::Dec {
                    operand: Operand::Register(Register::H),
                },
            },
            0x26 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x27 => Instruction {
                address,
                op: Opcode::Daa,
            },
            0x28 => Instruction {
                address,
                op: Opcode::JrCond {
                    condition: ConditionType::Zero,
                    offset: make_i8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x29 => Instruction {
                address,
                op: Opcode::Add16 {
                    register: Register::Hl,
                    operand: Operand::Register(Register::Hl),
                },
            },
            0x2a => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Register(Register::HlPlus)),
                },
            },
            0x2b => Instruction {
                address,
                op: Opcode::Dec16 {
                    register: Register::Hl,
                },
            },
            0x2c => Instruction {
                address,
                op: Opcode::Inc {
                    operand: Operand::Register(Register::L),
                },
            },
            0x2d => Instruction {
                address,
                op: Opcode::Dec {
                    operand: Operand::Register(Register::L),
                },
            },
            0x2e => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x2f => Instruction {
                address,
                op: Opcode::Cpl,
            },
            0x30 => Instruction {
                address,
                op: Opcode::JrCond {
                    condition: ConditionType::NotCarry,
                    offset: make_i8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x31 => Instruction {
                address,
                op: Opcode::Ld16 {
                    destination: Operand::Register(Register::Sp),
                    source: Operand::U16(make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    )),
                },
            },
            0x32 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::HlMinus)),
                    source: Operand::Register(Register::A),
                },
            },
            0x33 => Instruction {
                address,
                op: Opcode::Inc16 {
                    register: Register::Sp,
                },
            },
            0x34 => Instruction {
                address,
                op: Opcode::Inc {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x35 => Instruction {
                address,
                op: Opcode::Dec {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x36 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                    source: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x37 => Instruction {
                address,
                op: Opcode::Scf,
            },
            0x38 => Instruction {
                address,
                op: Opcode::JrCond {
                    condition: ConditionType::Carry,
                    offset: make_i8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x39 => Instruction {
                address,
                op: Opcode::Add16 {
                    register: Register::Hl,
                    operand: Operand::Register(Register::Sp),
                },
            },
            0x3a => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Register(Register::HlMinus)),
                },
            },
            0x3b => Instruction {
                address,
                op: Opcode::Dec16 {
                    register: Register::Sp,
                },
            },
            0x3c => Instruction {
                address,
                op: Opcode::Inc {
                    operand: Operand::Register(Register::A),
                },
            },
            0x3d => Instruction {
                address,
                op: Opcode::Dec {
                    operand: Operand::Register(Register::A),
                },
            },
            0x3e => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0x3f => Instruction {
                address,
                op: Opcode::Ccf,
            },
            0x40 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::Register(Register::B),
                },
            },
            0x41 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::Register(Register::C),
                },
            },
            0x42 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::Register(Register::D),
                },
            },
            0x43 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::Register(Register::E),
                },
            },
            0x44 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::Register(Register::H),
                },
            },
            0x45 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::Register(Register::L),
                },
            },
            0x46 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x47 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::B),
                    source: Operand::Register(Register::A),
                },
            },
            0x48 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::Register(Register::B),
                },
            },
            0x49 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::Register(Register::C),
                },
            },
            0x4a => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::Register(Register::D),
                },
            },
            0x4b => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::Register(Register::E),
                },
            },
            0x4c => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::Register(Register::H),
                },
            },
            0x4d => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::Register(Register::L),
                },
            },
            0x4e => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x4f => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::C),
                    source: Operand::Register(Register::A),
                },
            },
            0x50 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::Register(Register::B),
                },
            },
            0x51 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::Register(Register::C),
                },
            },
            0x52 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::Register(Register::D),
                },
            },
            0x53 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::Register(Register::E),
                },
            },
            0x54 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::Register(Register::H),
                },
            },
            0x55 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::Register(Register::L),
                },
            },
            0x56 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x57 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::D),
                    source: Operand::Register(Register::A),
                },
            },
            0x58 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::Register(Register::B),
                },
            },
            0x59 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::Register(Register::C),
                },
            },
            0x5a => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::Register(Register::D),
                },
            },
            0x5b => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::Register(Register::E),
                },
            },
            0x5c => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::Register(Register::H),
                },
            },
            0x5d => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::Register(Register::L),
                },
            },
            0x5e => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x5f => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::E),
                    source: Operand::Register(Register::A),
                },
            },
            0x60 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::Register(Register::B),
                },
            },
            0x61 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::Register(Register::C),
                },
            },
            0x62 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::Register(Register::D),
                },
            },
            0x63 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::Register(Register::E),
                },
            },
            0x64 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::Register(Register::H),
                },
            },
            0x65 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::Register(Register::L),
                },
            },
            0x66 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x67 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::H),
                    source: Operand::Register(Register::A),
                },
            },
            0x68 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::Register(Register::B),
                },
            },
            0x69 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::Register(Register::C),
                },
            },
            0x6a => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::Register(Register::D),
                },
            },
            0x6b => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::Register(Register::E),
                },
            },
            0x6c => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::Register(Register::H),
                },
            },
            0x6d => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::Register(Register::L),
                },
            },
            0x6e => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x6f => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::L),
                    source: Operand::Register(Register::A),
                },
            },
            0x70 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                    source: Operand::Register(Register::B),
                },
            },
            0x71 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                    source: Operand::Register(Register::C),
                },
            },
            0x72 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                    source: Operand::Register(Register::D),
                },
            },
            0x73 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                    source: Operand::Register(Register::E),
                },
            },
            0x74 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                    source: Operand::Register(Register::H),
                },
            },
            0x75 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                    source: Operand::Register(Register::L),
                },
            },
            0x76 => Instruction {
                address,
                op: Opcode::Halt,
            },
            0x77 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                    source: Operand::Register(Register::A),
                },
            },
            0x78 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Register(Register::B),
                },
            },
            0x79 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Register(Register::C),
                },
            },
            0x7a => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Register(Register::D),
                },
            },
            0x7b => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Register(Register::E),
                },
            },
            0x7c => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Register(Register::H),
                },
            },
            0x7d => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Register(Register::L),
                },
            },
            0x7e => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x7f => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Register(Register::A),
                },
            },
            0x80 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::Register(Register::B),
                },
            },
            0x81 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::Register(Register::C),
                },
            },
            0x82 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::Register(Register::D),
                },
            },
            0x83 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::Register(Register::E),
                },
            },
            0x84 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::Register(Register::H),
                },
            },
            0x85 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::Register(Register::L),
                },
            },
            0x86 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x87 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::Register(Register::A),
                },
            },
            0x88 => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::Register(Register::B),
                },
            },
            0x89 => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::Register(Register::C),
                },
            },
            0x8a => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::Register(Register::D),
                },
            },
            0x8b => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::Register(Register::E),
                },
            },
            0x8c => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::Register(Register::H),
                },
            },
            0x8d => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::Register(Register::L),
                },
            },
            0x8e => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x8f => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::Register(Register::A),
                },
            },
            0x90 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::Register(Register::B),
                },
            },
            0x91 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::Register(Register::C),
                },
            },
            0x92 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::Register(Register::D),
                },
            },
            0x93 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::Register(Register::E),
                },
            },
            0x94 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::Register(Register::H),
                },
            },
            0x95 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::Register(Register::L),
                },
            },
            0x96 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x97 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::Register(Register::A),
                },
            },
            0x98 => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::Register(Register::B),
                },
            },
            0x99 => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::Register(Register::C),
                },
            },
            0x9a => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::Register(Register::D),
                },
            },
            0x9b => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::Register(Register::E),
                },
            },
            0x9c => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::Register(Register::H),
                },
            },
            0x9d => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::Register(Register::L),
                },
            },
            0x9e => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x9f => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::Register(Register::A),
                },
            },
            0xa0 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::Register(Register::B),
                },
            },
            0xa1 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::Register(Register::C),
                },
            },
            0xa2 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::Register(Register::D),
                },
            },
            0xa3 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::Register(Register::E),
                },
            },
            0xa4 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::Register(Register::H),
                },
            },
            0xa5 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::Register(Register::L),
                },
            },
            0xa6 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xa7 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::Register(Register::A),
                },
            },
            0xa8 => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::Register(Register::B),
                },
            },
            0xa9 => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::Register(Register::C),
                },
            },
            0xaa => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::Register(Register::D),
                },
            },
            0xab => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::Register(Register::E),
                },
            },
            0xac => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::Register(Register::H),
                },
            },
            0xad => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::Register(Register::L),
                },
            },
            0xae => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xaf => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::Register(Register::A),
                },
            },
            0xb0 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::Register(Register::B),
                },
            },
            0xb1 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::Register(Register::C),
                },
            },
            0xb2 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::Register(Register::D),
                },
            },
            0xb3 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::Register(Register::E),
                },
            },
            0xb4 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::Register(Register::H),
                },
            },
            0xb5 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::Register(Register::L),
                },
            },
            0xb6 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xb7 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::Register(Register::A),
                },
            },
            0xb8 => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::Register(Register::B),
                },
            },
            0xb9 => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::Register(Register::C),
                },
            },
            0xba => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::Register(Register::D),
                },
            },
            0xbb => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::Register(Register::E),
                },
            },
            0xbc => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::Register(Register::H),
                },
            },
            0xbd => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::Register(Register::L),
                },
            },
            0xbe => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xbf => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::Register(Register::A),
                },
            },
            0xc0 => Instruction {
                address,
                op: Opcode::RetCond {
                    condition: ConditionType::NonZero,
                },
            },
            0xc1 => Instruction {
                address,
                op: Opcode::Pop {
                    register: Register::Bc,
                },
            },
            0xc2 => Instruction {
                address,
                op: Opcode::JpCond {
                    condition: ConditionType::NonZero,
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xc3 => Instruction {
                address,
                op: Opcode::Jp {
                    destination: Operand::U16(make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    )),
                },
            },
            0xc4 => Instruction {
                address,
                op: Opcode::CallCond {
                    condition: ConditionType::NonZero,
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xc5 => Instruction {
                address,
                op: Opcode::Push {
                    register: Register::Bc,
                },
            },
            0xc6 => Instruction {
                address,
                op: Opcode::Add8 {
                    operand: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0xc7 => Instruction {
                address,
                op: Opcode::Rst { vector: 0x00 },
            },
            0xc8 => Instruction {
                address,
                op: Opcode::RetCond {
                    condition: ConditionType::Zero,
                },
            },
            0xc9 => Instruction {
                address,
                op: Opcode::Ret,
            },
            0xca => Instruction {
                address,
                op: Opcode::JpCond {
                    condition: ConditionType::Zero,
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xcb => Self::make_cb_instruction(address, memory_bus),
            0xcc => Instruction {
                address,
                op: Opcode::CallCond {
                    condition: ConditionType::Zero,
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xcd => Instruction {
                address,
                op: Opcode::Call {
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xce => Instruction {
                address,
                op: Opcode::Adc {
                    operand: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0xcf => Instruction {
                address,
                op: Opcode::Rst { vector: 0x08 },
            },
            0xd0 => Instruction {
                address,
                op: Opcode::RetCond {
                    condition: ConditionType::NotCarry,
                },
            },
            0xd1 => Instruction {
                address,
                op: Opcode::Pop {
                    register: Register::De,
                },
            },
            0xd2 => Instruction {
                address,
                op: Opcode::JpCond {
                    condition: ConditionType::NotCarry,
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xd3 => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xd3 },
            },
            0xd4 => Instruction {
                address,
                op: Opcode::CallCond {
                    condition: ConditionType::NotCarry,
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xd5 => Instruction {
                address,
                op: Opcode::Push {
                    register: Register::De,
                },
            },
            0xd6 => Instruction {
                address,
                op: Opcode::Sub {
                    operand: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0xd7 => Instruction {
                address,
                op: Opcode::Rst { vector: 0x10 },
            },
            0xd8 => Instruction {
                address,
                op: Opcode::RetCond {
                    condition: ConditionType::Carry,
                },
            },
            0xd9 => Instruction {
                address,
                op: Opcode::Reti,
            },
            0xda => Instruction {
                address,
                op: Opcode::JpCond {
                    condition: ConditionType::Carry,
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xdb => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xdb },
            },
            0xdc => Instruction {
                address,
                op: Opcode::CallCond {
                    condition: ConditionType::Carry,
                    destination: make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ),
                },
            },
            0xdd => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xdd },
            },
            0xde => Instruction {
                address,
                op: Opcode::Sbc {
                    operand: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0xdf => Instruction {
                address,
                op: Opcode::Rst { vector: 0x18 },
            },
            0xe0 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Ff00Offset(
                        memory_bus.read_u8(address.wrapping_add(1)),
                    )),
                    source: Operand::Register(Register::A),
                },
            },
            0xe1 => Instruction {
                address,
                op: Opcode::Pop {
                    register: Register::Hl,
                },
            },
            0xe2 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Ff00PlusC),
                    source: Operand::Register(Register::A),
                },
            },
            0xe3 => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xe3 },
            },
            0xe4 => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xe4 },
            },
            0xe5 => Instruction {
                address,
                op: Opcode::Push {
                    register: Register::Hl,
                },
            },
            0xe6 => Instruction {
                address,
                op: Opcode::And {
                    operand: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0xe7 => Instruction {
                address,
                op: Opcode::Rst { vector: 0x20 },
            },
            0xe8 => Instruction {
                address,
                op: Opcode::Add16 {
                    register: Register::Sp,
                    operand: Operand::I8(make_i8(memory_bus.read_u8(address.wrapping_add(1)))),
                },
            },
            0xe9 => Instruction {
                address,
                op: Opcode::Jp {
                    destination: Operand::Register(Register::Hl),
                },
            },
            0xea => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Address(make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ))),
                    source: Operand::Register(Register::A),
                },
            },
            0xeb => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xeb },
            },
            0xec => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xec },
            },
            0xed => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xed },
            },
            0xee => Instruction {
                address,
                op: Opcode::Xor {
                    operand: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0xef => Instruction {
                address,
                op: Opcode::Rst { vector: 0x28 },
            },
            0xf0 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Ff00Offset(
                        memory_bus.read_u8(address.wrapping_add(1)),
                    )),
                },
            },
            0xf1 => Instruction {
                address,
                op: Opcode::Pop {
                    register: Register::Af,
                },
            },
            0xf2 => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Ff00PlusC),
                },
            },
            0xf3 => Instruction {
                address,
                op: Opcode::Di,
            },
            0xf4 => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xe4 },
            },
            0xf5 => Instruction {
                address,
                op: Opcode::Push {
                    register: Register::Af,
                },
            },
            0xf6 => Instruction {
                address,
                op: Opcode::Or {
                    operand: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0xf7 => Instruction {
                address,
                op: Opcode::Rst { vector: 0x30 },
            },
            0xf8 => Instruction {
                address,
                op: Opcode::Ld16 {
                    destination: Operand::Register(Register::Hl),
                    source: Operand::StackOffset(make_i8(
                        memory_bus.read_u8(address.wrapping_add(1)),
                    )),
                },
            },
            0xf9 => Instruction {
                address,
                op: Opcode::Ld16 {
                    destination: Operand::Register(Register::Sp),
                    source: Operand::Register(Register::Hl),
                },
            },
            0xfa => Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Address(make_u16(
                        memory_bus.read_u8(address.wrapping_add(1)),
                        memory_bus.read_u8(address.wrapping_add(2)),
                    ))),
                },
            },
            0xfb => Instruction {
                address,
                op: Opcode::Ei,
            },
            0xfc => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xfc },
            },
            0xfd => Instruction {
                address,
                op: Opcode::Unknown { opcode: 0xfd },
            },
            0xfe => Instruction {
                address,
                op: Opcode::Cp {
                    operand: Operand::U8(memory_bus.read_u8(address.wrapping_add(1))),
                },
            },
            0xff => Instruction {
                address,
                op: Opcode::Rst { vector: 0x38 },
            },
        }
    }

    fn make_cb_instruction(address: u16, memory_bus: &MemoryBus) -> Instruction {
        let op = memory_bus.read_u8(address.wrapping_add(1));
        #[allow(clippy::match_same_arms)]
        match op {
            0x00 => Instruction {
                address,
                op: Opcode::Rlc {
                    operand: Operand::Register(Register::B),
                },
            },
            0x01 => Instruction {
                address,
                op: Opcode::Rlc {
                    operand: Operand::Register(Register::C),
                },
            },
            0x02 => Instruction {
                address,
                op: Opcode::Rlc {
                    operand: Operand::Register(Register::D),
                },
            },
            0x03 => Instruction {
                address,
                op: Opcode::Rlc {
                    operand: Operand::Register(Register::E),
                },
            },
            0x04 => Instruction {
                address,
                op: Opcode::Rlc {
                    operand: Operand::Register(Register::H),
                },
            },
            0x05 => Instruction {
                address,
                op: Opcode::Rlc {
                    operand: Operand::Register(Register::L),
                },
            },
            0x06 => Instruction {
                address,
                op: Opcode::Rlc {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x07 => Instruction {
                address,
                op: Opcode::Rlc {
                    operand: Operand::Register(Register::A),
                },
            },
            0x08 => Instruction {
                address,
                op: Opcode::Rrc {
                    operand: Operand::Register(Register::B),
                },
            },
            0x09 => Instruction {
                address,
                op: Opcode::Rrc {
                    operand: Operand::Register(Register::C),
                },
            },
            0x0a => Instruction {
                address,
                op: Opcode::Rrc {
                    operand: Operand::Register(Register::D),
                },
            },
            0x0b => Instruction {
                address,
                op: Opcode::Rrc {
                    operand: Operand::Register(Register::E),
                },
            },
            0x0c => Instruction {
                address,
                op: Opcode::Rrc {
                    operand: Operand::Register(Register::H),
                },
            },
            0x0d => Instruction {
                address,
                op: Opcode::Rrc {
                    operand: Operand::Register(Register::L),
                },
            },
            0x0e => Instruction {
                address,
                op: Opcode::Rrc {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x0f => Instruction {
                address,
                op: Opcode::Rrc {
                    operand: Operand::Register(Register::A),
                },
            },
            0x10 => Instruction {
                address,
                op: Opcode::Rl {
                    operand: Operand::Register(Register::B),
                },
            },
            0x11 => Instruction {
                address,
                op: Opcode::Rl {
                    operand: Operand::Register(Register::C),
                },
            },
            0x12 => Instruction {
                address,
                op: Opcode::Rl {
                    operand: Operand::Register(Register::D),
                },
            },
            0x13 => Instruction {
                address,
                op: Opcode::Rl {
                    operand: Operand::Register(Register::E),
                },
            },
            0x14 => Instruction {
                address,
                op: Opcode::Rl {
                    operand: Operand::Register(Register::H),
                },
            },
            0x15 => Instruction {
                address,
                op: Opcode::Rl {
                    operand: Operand::Register(Register::L),
                },
            },
            0x16 => Instruction {
                address,
                op: Opcode::Rl {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x17 => Instruction {
                address,
                op: Opcode::Rl {
                    operand: Operand::Register(Register::A),
                },
            },
            0x18 => Instruction {
                address,
                op: Opcode::Rr {
                    operand: Operand::Register(Register::B),
                },
            },
            0x19 => Instruction {
                address,
                op: Opcode::Rr {
                    operand: Operand::Register(Register::C),
                },
            },
            0x1a => Instruction {
                address,
                op: Opcode::Rr {
                    operand: Operand::Register(Register::D),
                },
            },
            0x1b => Instruction {
                address,
                op: Opcode::Rr {
                    operand: Operand::Register(Register::E),
                },
            },
            0x1c => Instruction {
                address,
                op: Opcode::Rr {
                    operand: Operand::Register(Register::H),
                },
            },
            0x1d => Instruction {
                address,
                op: Opcode::Rr {
                    operand: Operand::Register(Register::L),
                },
            },
            0x1e => Instruction {
                address,
                op: Opcode::Rr {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x1f => Instruction {
                address,
                op: Opcode::Rr {
                    operand: Operand::Register(Register::A),
                },
            },
            0x20 => Instruction {
                address,
                op: Opcode::Sla {
                    operand: Operand::Register(Register::B),
                },
            },
            0x21 => Instruction {
                address,
                op: Opcode::Sla {
                    operand: Operand::Register(Register::C),
                },
            },
            0x22 => Instruction {
                address,
                op: Opcode::Sla {
                    operand: Operand::Register(Register::D),
                },
            },
            0x23 => Instruction {
                address,
                op: Opcode::Sla {
                    operand: Operand::Register(Register::E),
                },
            },
            0x24 => Instruction {
                address,
                op: Opcode::Sla {
                    operand: Operand::Register(Register::H),
                },
            },
            0x25 => Instruction {
                address,
                op: Opcode::Sla {
                    operand: Operand::Register(Register::L),
                },
            },
            0x26 => Instruction {
                address,
                op: Opcode::Sla {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x27 => Instruction {
                address,
                op: Opcode::Sla {
                    operand: Operand::Register(Register::A),
                },
            },
            0x28 => Instruction {
                address,
                op: Opcode::Sra {
                    operand: Operand::Register(Register::B),
                },
            },
            0x29 => Instruction {
                address,
                op: Opcode::Sra {
                    operand: Operand::Register(Register::C),
                },
            },
            0x2a => Instruction {
                address,
                op: Opcode::Sra {
                    operand: Operand::Register(Register::D),
                },
            },
            0x2b => Instruction {
                address,
                op: Opcode::Sra {
                    operand: Operand::Register(Register::E),
                },
            },
            0x2c => Instruction {
                address,
                op: Opcode::Sra {
                    operand: Operand::Register(Register::H),
                },
            },
            0x2d => Instruction {
                address,
                op: Opcode::Sra {
                    operand: Operand::Register(Register::L),
                },
            },
            0x2e => Instruction {
                address,
                op: Opcode::Sra {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x2f => Instruction {
                address,
                op: Opcode::Sra {
                    operand: Operand::Register(Register::A),
                },
            },
            0x30 => Instruction {
                address,
                op: Opcode::Swap {
                    operand: Operand::Register(Register::B),
                },
            },
            0x31 => Instruction {
                address,
                op: Opcode::Swap {
                    operand: Operand::Register(Register::C),
                },
            },
            0x32 => Instruction {
                address,
                op: Opcode::Swap {
                    operand: Operand::Register(Register::D),
                },
            },
            0x33 => Instruction {
                address,
                op: Opcode::Swap {
                    operand: Operand::Register(Register::E),
                },
            },
            0x34 => Instruction {
                address,
                op: Opcode::Swap {
                    operand: Operand::Register(Register::H),
                },
            },
            0x35 => Instruction {
                address,
                op: Opcode::Swap {
                    operand: Operand::Register(Register::L),
                },
            },
            0x36 => Instruction {
                address,
                op: Opcode::Swap {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x37 => Instruction {
                address,
                op: Opcode::Swap {
                    operand: Operand::Register(Register::A),
                },
            },
            0x38 => Instruction {
                address,
                op: Opcode::Srl {
                    operand: Operand::Register(Register::B),
                },
            },
            0x39 => Instruction {
                address,
                op: Opcode::Srl {
                    operand: Operand::Register(Register::C),
                },
            },
            0x3a => Instruction {
                address,
                op: Opcode::Srl {
                    operand: Operand::Register(Register::D),
                },
            },
            0x3b => Instruction {
                address,
                op: Opcode::Srl {
                    operand: Operand::Register(Register::E),
                },
            },
            0x3c => Instruction {
                address,
                op: Opcode::Srl {
                    operand: Operand::Register(Register::H),
                },
            },
            0x3d => Instruction {
                address,
                op: Opcode::Srl {
                    operand: Operand::Register(Register::L),
                },
            },
            0x3e => Instruction {
                address,
                op: Opcode::Srl {
                    operand: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x3f => Instruction {
                address,
                op: Opcode::Srl {
                    operand: Operand::Register(Register::A),
                },
            },
            0x40 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 0,
                    destination: Operand::Register(Register::B),
                },
            },
            0x41 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 0,
                    destination: Operand::Register(Register::C),
                },
            },
            0x42 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 0,
                    destination: Operand::Register(Register::D),
                },
            },
            0x43 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 0,
                    destination: Operand::Register(Register::E),
                },
            },
            0x44 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 0,
                    destination: Operand::Register(Register::H),
                },
            },
            0x45 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 0,
                    destination: Operand::Register(Register::L),
                },
            },
            0x46 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 0,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x47 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 0,
                    destination: Operand::Register(Register::A),
                },
            },
            0x48 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 1,
                    destination: Operand::Register(Register::B),
                },
            },
            0x49 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 1,
                    destination: Operand::Register(Register::C),
                },
            },
            0x4a => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 1,
                    destination: Operand::Register(Register::D),
                },
            },
            0x4b => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 1,
                    destination: Operand::Register(Register::E),
                },
            },
            0x4c => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 1,
                    destination: Operand::Register(Register::H),
                },
            },
            0x4d => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 1,
                    destination: Operand::Register(Register::L),
                },
            },
            0x4e => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 1,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x4f => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 1,
                    destination: Operand::Register(Register::A),
                },
            },
            0x50 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 2,
                    destination: Operand::Register(Register::B),
                },
            },
            0x51 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 2,
                    destination: Operand::Register(Register::C),
                },
            },
            0x52 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 2,
                    destination: Operand::Register(Register::D),
                },
            },
            0x53 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 2,
                    destination: Operand::Register(Register::E),
                },
            },
            0x54 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 2,
                    destination: Operand::Register(Register::H),
                },
            },
            0x55 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 2,
                    destination: Operand::Register(Register::L),
                },
            },
            0x56 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 2,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x57 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 2,
                    destination: Operand::Register(Register::A),
                },
            },
            0x58 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 3,
                    destination: Operand::Register(Register::B),
                },
            },
            0x59 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 3,
                    destination: Operand::Register(Register::C),
                },
            },
            0x5a => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 3,
                    destination: Operand::Register(Register::D),
                },
            },
            0x5b => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 3,
                    destination: Operand::Register(Register::E),
                },
            },
            0x5c => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 3,
                    destination: Operand::Register(Register::H),
                },
            },
            0x5d => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 3,
                    destination: Operand::Register(Register::L),
                },
            },
            0x5e => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 3,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x5f => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 3,
                    destination: Operand::Register(Register::A),
                },
            },
            0x60 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 4,
                    destination: Operand::Register(Register::B),
                },
            },
            0x61 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 4,
                    destination: Operand::Register(Register::C),
                },
            },
            0x62 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 4,
                    destination: Operand::Register(Register::D),
                },
            },
            0x63 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 4,
                    destination: Operand::Register(Register::E),
                },
            },
            0x64 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 4,
                    destination: Operand::Register(Register::H),
                },
            },
            0x65 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 4,
                    destination: Operand::Register(Register::L),
                },
            },
            0x66 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 4,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x67 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 4,
                    destination: Operand::Register(Register::A),
                },
            },
            0x68 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 5,
                    destination: Operand::Register(Register::B),
                },
            },
            0x69 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 5,
                    destination: Operand::Register(Register::C),
                },
            },
            0x6a => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 5,
                    destination: Operand::Register(Register::D),
                },
            },
            0x6b => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 5,
                    destination: Operand::Register(Register::E),
                },
            },
            0x6c => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 5,
                    destination: Operand::Register(Register::H),
                },
            },
            0x6d => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 5,
                    destination: Operand::Register(Register::L),
                },
            },
            0x6e => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 5,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x6f => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 5,
                    destination: Operand::Register(Register::A),
                },
            },
            0x70 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 6,
                    destination: Operand::Register(Register::B),
                },
            },
            0x71 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 6,
                    destination: Operand::Register(Register::C),
                },
            },
            0x72 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 6,
                    destination: Operand::Register(Register::D),
                },
            },
            0x73 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 6,
                    destination: Operand::Register(Register::E),
                },
            },
            0x74 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 6,
                    destination: Operand::Register(Register::H),
                },
            },
            0x75 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 6,
                    destination: Operand::Register(Register::L),
                },
            },
            0x76 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 6,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x77 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 6,
                    destination: Operand::Register(Register::A),
                },
            },
            0x78 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 7,
                    destination: Operand::Register(Register::B),
                },
            },
            0x79 => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 7,
                    destination: Operand::Register(Register::C),
                },
            },
            0x7a => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 7,
                    destination: Operand::Register(Register::D),
                },
            },
            0x7b => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 7,
                    destination: Operand::Register(Register::E),
                },
            },
            0x7c => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 7,
                    destination: Operand::Register(Register::H),
                },
            },
            0x7d => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 7,
                    destination: Operand::Register(Register::L),
                },
            },
            0x7e => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 7,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x7f => Instruction {
                address,
                op: Opcode::Bit {
                    bit: 7,
                    destination: Operand::Register(Register::A),
                },
            },
            0x80 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 0,
                    destination: Operand::Register(Register::B),
                },
            },
            0x81 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 0,
                    destination: Operand::Register(Register::C),
                },
            },
            0x82 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 0,
                    destination: Operand::Register(Register::D),
                },
            },
            0x83 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 0,
                    destination: Operand::Register(Register::E),
                },
            },
            0x84 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 0,
                    destination: Operand::Register(Register::H),
                },
            },
            0x85 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 0,
                    destination: Operand::Register(Register::L),
                },
            },
            0x86 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 0,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x87 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 0,
                    destination: Operand::Register(Register::A),
                },
            },
            0x88 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 1,
                    destination: Operand::Register(Register::B),
                },
            },
            0x89 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 1,
                    destination: Operand::Register(Register::C),
                },
            },
            0x8a => Instruction {
                address,
                op: Opcode::Res {
                    bit: 1,
                    destination: Operand::Register(Register::D),
                },
            },
            0x8b => Instruction {
                address,
                op: Opcode::Res {
                    bit: 1,
                    destination: Operand::Register(Register::E),
                },
            },
            0x8c => Instruction {
                address,
                op: Opcode::Res {
                    bit: 1,
                    destination: Operand::Register(Register::H),
                },
            },
            0x8d => Instruction {
                address,
                op: Opcode::Res {
                    bit: 1,
                    destination: Operand::Register(Register::L),
                },
            },
            0x8e => Instruction {
                address,
                op: Opcode::Res {
                    bit: 1,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x8f => Instruction {
                address,
                op: Opcode::Res {
                    bit: 1,
                    destination: Operand::Register(Register::A),
                },
            },
            0x90 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 2,
                    destination: Operand::Register(Register::B),
                },
            },
            0x91 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 2,
                    destination: Operand::Register(Register::C),
                },
            },
            0x92 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 2,
                    destination: Operand::Register(Register::D),
                },
            },
            0x93 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 2,
                    destination: Operand::Register(Register::E),
                },
            },
            0x94 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 2,
                    destination: Operand::Register(Register::H),
                },
            },
            0x95 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 2,
                    destination: Operand::Register(Register::L),
                },
            },
            0x96 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 2,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x97 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 2,
                    destination: Operand::Register(Register::A),
                },
            },
            0x98 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 3,
                    destination: Operand::Register(Register::B),
                },
            },
            0x99 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 3,
                    destination: Operand::Register(Register::C),
                },
            },
            0x9a => Instruction {
                address,
                op: Opcode::Res {
                    bit: 3,
                    destination: Operand::Register(Register::D),
                },
            },
            0x9b => Instruction {
                address,
                op: Opcode::Res {
                    bit: 3,
                    destination: Operand::Register(Register::E),
                },
            },
            0x9c => Instruction {
                address,
                op: Opcode::Res {
                    bit: 3,
                    destination: Operand::Register(Register::H),
                },
            },
            0x9d => Instruction {
                address,
                op: Opcode::Res {
                    bit: 3,
                    destination: Operand::Register(Register::L),
                },
            },
            0x9e => Instruction {
                address,
                op: Opcode::Res {
                    bit: 3,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0x9f => Instruction {
                address,
                op: Opcode::Res {
                    bit: 3,
                    destination: Operand::Register(Register::A),
                },
            },
            0xa0 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 4,
                    destination: Operand::Register(Register::B),
                },
            },
            0xa1 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 4,
                    destination: Operand::Register(Register::C),
                },
            },
            0xa2 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 4,
                    destination: Operand::Register(Register::D),
                },
            },
            0xa3 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 4,
                    destination: Operand::Register(Register::E),
                },
            },
            0xa4 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 4,
                    destination: Operand::Register(Register::H),
                },
            },
            0xa5 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 4,
                    destination: Operand::Register(Register::L),
                },
            },
            0xa6 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 4,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xa7 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 4,
                    destination: Operand::Register(Register::A),
                },
            },
            0xa8 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 5,
                    destination: Operand::Register(Register::B),
                },
            },
            0xa9 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 5,
                    destination: Operand::Register(Register::C),
                },
            },
            0xaa => Instruction {
                address,
                op: Opcode::Res {
                    bit: 5,
                    destination: Operand::Register(Register::D),
                },
            },
            0xab => Instruction {
                address,
                op: Opcode::Res {
                    bit: 5,
                    destination: Operand::Register(Register::E),
                },
            },
            0xac => Instruction {
                address,
                op: Opcode::Res {
                    bit: 5,
                    destination: Operand::Register(Register::H),
                },
            },
            0xad => Instruction {
                address,
                op: Opcode::Res {
                    bit: 5,
                    destination: Operand::Register(Register::L),
                },
            },
            0xae => Instruction {
                address,
                op: Opcode::Res {
                    bit: 5,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xaf => Instruction {
                address,
                op: Opcode::Res {
                    bit: 5,
                    destination: Operand::Register(Register::A),
                },
            },
            0xb0 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 6,
                    destination: Operand::Register(Register::B),
                },
            },
            0xb1 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 6,
                    destination: Operand::Register(Register::C),
                },
            },
            0xb2 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 6,
                    destination: Operand::Register(Register::D),
                },
            },
            0xb3 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 6,
                    destination: Operand::Register(Register::E),
                },
            },
            0xb4 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 6,
                    destination: Operand::Register(Register::H),
                },
            },
            0xb5 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 6,
                    destination: Operand::Register(Register::L),
                },
            },
            0xb6 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 6,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xb7 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 6,
                    destination: Operand::Register(Register::A),
                },
            },
            0xb8 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 7,
                    destination: Operand::Register(Register::B),
                },
            },
            0xb9 => Instruction {
                address,
                op: Opcode::Res {
                    bit: 7,
                    destination: Operand::Register(Register::C),
                },
            },
            0xba => Instruction {
                address,
                op: Opcode::Res {
                    bit: 7,
                    destination: Operand::Register(Register::D),
                },
            },
            0xbb => Instruction {
                address,
                op: Opcode::Res {
                    bit: 7,
                    destination: Operand::Register(Register::E),
                },
            },
            0xbc => Instruction {
                address,
                op: Opcode::Res {
                    bit: 7,
                    destination: Operand::Register(Register::H),
                },
            },
            0xbd => Instruction {
                address,
                op: Opcode::Res {
                    bit: 7,
                    destination: Operand::Register(Register::L),
                },
            },
            0xbe => Instruction {
                address,
                op: Opcode::Res {
                    bit: 7,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xbf => Instruction {
                address,
                op: Opcode::Res {
                    bit: 7,
                    destination: Operand::Register(Register::A),
                },
            },
            0xc0 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 0,
                    destination: Operand::Register(Register::B),
                },
            },
            0xc1 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 0,
                    destination: Operand::Register(Register::C),
                },
            },
            0xc2 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 0,
                    destination: Operand::Register(Register::D),
                },
            },
            0xc3 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 0,
                    destination: Operand::Register(Register::E),
                },
            },
            0xc4 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 0,
                    destination: Operand::Register(Register::H),
                },
            },
            0xc5 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 0,
                    destination: Operand::Register(Register::L),
                },
            },
            0xc6 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 0,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xc7 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 0,
                    destination: Operand::Register(Register::A),
                },
            },
            0xc8 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 1,
                    destination: Operand::Register(Register::B),
                },
            },
            0xc9 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 1,
                    destination: Operand::Register(Register::C),
                },
            },
            0xca => Instruction {
                address,
                op: Opcode::Set {
                    bit: 1,
                    destination: Operand::Register(Register::D),
                },
            },
            0xcb => Instruction {
                address,
                op: Opcode::Set {
                    bit: 1,
                    destination: Operand::Register(Register::E),
                },
            },
            0xcc => Instruction {
                address,
                op: Opcode::Set {
                    bit: 1,
                    destination: Operand::Register(Register::H),
                },
            },
            0xcd => Instruction {
                address,
                op: Opcode::Set {
                    bit: 1,
                    destination: Operand::Register(Register::L),
                },
            },
            0xce => Instruction {
                address,
                op: Opcode::Set {
                    bit: 1,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xcf => Instruction {
                address,
                op: Opcode::Set {
                    bit: 1,
                    destination: Operand::Register(Register::A),
                },
            },
            0xd0 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 2,
                    destination: Operand::Register(Register::B),
                },
            },
            0xd1 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 2,
                    destination: Operand::Register(Register::C),
                },
            },
            0xd2 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 2,
                    destination: Operand::Register(Register::D),
                },
            },
            0xd3 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 2,
                    destination: Operand::Register(Register::E),
                },
            },
            0xd4 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 2,
                    destination: Operand::Register(Register::H),
                },
            },
            0xd5 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 2,
                    destination: Operand::Register(Register::L),
                },
            },
            0xd6 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 2,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xd7 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 2,
                    destination: Operand::Register(Register::A),
                },
            },
            0xd8 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 3,
                    destination: Operand::Register(Register::B),
                },
            },
            0xd9 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 3,
                    destination: Operand::Register(Register::C),
                },
            },
            0xda => Instruction {
                address,
                op: Opcode::Set {
                    bit: 3,
                    destination: Operand::Register(Register::D),
                },
            },
            0xdb => Instruction {
                address,
                op: Opcode::Set {
                    bit: 3,
                    destination: Operand::Register(Register::E),
                },
            },
            0xdc => Instruction {
                address,
                op: Opcode::Set {
                    bit: 3,
                    destination: Operand::Register(Register::H),
                },
            },
            0xdd => Instruction {
                address,
                op: Opcode::Set {
                    bit: 3,
                    destination: Operand::Register(Register::L),
                },
            },
            0xde => Instruction {
                address,
                op: Opcode::Set {
                    bit: 3,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xdf => Instruction {
                address,
                op: Opcode::Set {
                    bit: 3,
                    destination: Operand::Register(Register::A),
                },
            },
            0xe0 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 4,
                    destination: Operand::Register(Register::B),
                },
            },
            0xe1 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 4,
                    destination: Operand::Register(Register::C),
                },
            },
            0xe2 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 4,
                    destination: Operand::Register(Register::D),
                },
            },
            0xe3 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 4,
                    destination: Operand::Register(Register::E),
                },
            },
            0xe4 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 4,
                    destination: Operand::Register(Register::H),
                },
            },
            0xe5 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 4,
                    destination: Operand::Register(Register::L),
                },
            },
            0xe6 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 4,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xe7 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 4,
                    destination: Operand::Register(Register::A),
                },
            },
            0xe8 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 5,
                    destination: Operand::Register(Register::B),
                },
            },
            0xe9 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 5,
                    destination: Operand::Register(Register::C),
                },
            },
            0xea => Instruction {
                address,
                op: Opcode::Set {
                    bit: 5,
                    destination: Operand::Register(Register::D),
                },
            },
            0xeb => Instruction {
                address,
                op: Opcode::Set {
                    bit: 5,
                    destination: Operand::Register(Register::E),
                },
            },
            0xec => Instruction {
                address,
                op: Opcode::Set {
                    bit: 5,
                    destination: Operand::Register(Register::H),
                },
            },
            0xed => Instruction {
                address,
                op: Opcode::Set {
                    bit: 5,
                    destination: Operand::Register(Register::L),
                },
            },
            0xee => Instruction {
                address,
                op: Opcode::Set {
                    bit: 5,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xef => Instruction {
                address,
                op: Opcode::Set {
                    bit: 5,
                    destination: Operand::Register(Register::A),
                },
            },
            0xf0 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 6,
                    destination: Operand::Register(Register::B),
                },
            },
            0xf1 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 6,
                    destination: Operand::Register(Register::C),
                },
            },
            0xf2 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 6,
                    destination: Operand::Register(Register::D),
                },
            },
            0xf3 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 6,
                    destination: Operand::Register(Register::E),
                },
            },
            0xf4 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 6,
                    destination: Operand::Register(Register::H),
                },
            },
            0xf5 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 6,
                    destination: Operand::Register(Register::L),
                },
            },
            0xf6 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 6,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xf7 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 6,
                    destination: Operand::Register(Register::A),
                },
            },
            0xf8 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 7,
                    destination: Operand::Register(Register::B),
                },
            },
            0xf9 => Instruction {
                address,
                op: Opcode::Set {
                    bit: 7,
                    destination: Operand::Register(Register::C),
                },
            },
            0xfa => Instruction {
                address,
                op: Opcode::Set {
                    bit: 7,
                    destination: Operand::Register(Register::D),
                },
            },
            0xfb => Instruction {
                address,
                op: Opcode::Set {
                    bit: 7,
                    destination: Operand::Register(Register::E),
                },
            },
            0xfc => Instruction {
                address,
                op: Opcode::Set {
                    bit: 7,
                    destination: Operand::Register(Register::H),
                },
            },
            0xfd => Instruction {
                address,
                op: Opcode::Set {
                    bit: 7,
                    destination: Operand::Register(Register::L),
                },
            },
            0xfe => Instruction {
                address,
                op: Opcode::Set {
                    bit: 7,
                    destination: Operand::Deref(DerefOperand::Register(Register::Hl)),
                },
            },
            0xff => Instruction {
                address,
                op: Opcode::Set {
                    bit: 7,
                    destination: Operand::Register(Register::A),
                },
            },
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let size = self.op.size();
        write!(
            f,
            "0x{:04x} - {} (size = {})",
            self.address,
            self.op.print(self.address + u16::from(size)),
            size
        )
    }
}
