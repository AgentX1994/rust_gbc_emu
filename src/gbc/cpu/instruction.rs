use core::{fmt, panic};
use std::{fmt::Display, mem, u16};

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
    Unknown,
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

impl Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Opcode::Unknown => write!(f, "Unknown"),
            Opcode::Nop => write!(f, "nop"),
            Opcode::Stop => write!(f, "stop"),
            Opcode::Halt => write!(f, "halt"),
            Opcode::Ld8 {
                destination,
                source,
            } => write!(f, "ld {} {}", destination, source),
            Opcode::Ld16 {
                destination,
                source,
            } => write!(f, "ld {} {}", destination, source),
            Opcode::Jp { destination } => write!(f, "jp {}", destination),
            Opcode::JpCond {
                condition,
                destination,
            } => write!(f, "jp {},{:#x}", condition, destination),
            Opcode::Jr { offset } => write!(f, "jr {:#x}", offset),
            Opcode::JrCond { condition, offset } => write!(f, "jr {},{}", condition, offset),
            Opcode::Call { destination } => write!(f, "call {:#x}", destination),
            Opcode::CallCond {
                condition,
                destination,
            } => write!(f, "call {},{:#x}", condition, destination),
            Opcode::Ret => write!(f, "ret"),
            Opcode::RetCond { condition } => write!(f, "ret {}", condition),
            Opcode::Reti => write!(f, "reti"),
            Opcode::Pop { register } => write!(f, "pop {}", register),
            Opcode::Push { register } => write!(f, "push {}", register),
            Opcode::Rst { vector } => write!(f, "rst {:#x}", vector),
            Opcode::Bit { bit, destination } => write!(f, "bit {},{}", bit, destination),
            Opcode::Res { bit, destination } => write!(f, "res {},{}", bit, destination),
            Opcode::Set { bit, destination } => write!(f, "set {},{}", bit, destination),
            Opcode::Add8 { operand } => write!(f, "add A,{}", operand),
            Opcode::Add16 { register, operand } => write!(f, "add {},{}", register, operand),
            Opcode::Inc { operand } => write!(f, "inc {}", operand),
            Opcode::Inc16 { register } => write!(f, "inc {}", register),
            Opcode::Dec { operand } => write!(f, "dec {}", operand),
            Opcode::Dec16 { register } => write!(f, "dec {}", register),
            Opcode::Adc { operand } => write!(f, "adc A,{}", operand),
            Opcode::Sub { operand } => write!(f, "sub {}", operand),
            Opcode::Sbc { operand } => write!(f, "sbc A,{}", operand),
            Opcode::And { operand } => write!(f, "and {}", operand),
            Opcode::Xor { operand } => write!(f, "xor {}", operand),
            Opcode::Or { operand } => write!(f, "or {}", operand),
            Opcode::Cp { operand } => write!(f, "cp {}", operand),
            Opcode::Cpl => write!(f, "cpl"),
            Opcode::Daa => write!(f, "daa"),
            Opcode::Rlca => write!(f, "rlca"),
            Opcode::Rla => write!(f, "rla"),
            Opcode::Rrca => write!(f, "rrca"),
            Opcode::Rra => write!(f, "rra"),
            Opcode::Rlc { operand: register } => write!(f, "rlc {}", register),
            Opcode::Rl { operand: register } => write!(f, "rl {}", register),
            Opcode::Rrc { operand: register } => write!(f, "rrc {}", register),
            Opcode::Rr { operand: register } => write!(f, "rr {}", register),
            Opcode::Sla { operand: register } => write!(f, "sla {}", register),
            Opcode::Swap { operand: register } => write!(f, "swap {}", register),
            Opcode::Sra { operand: register } => write!(f, "sra {}", register),
            Opcode::Srl { operand: register } => write!(f, "Srl {}", register),
            Opcode::Scf => write!(f, "scf"),
            Opcode::Ccf => write!(f, "ccf"),
            Opcode::Di => write!(f, "di"),
            Opcode::Ei => write!(f, "ei"),
        }
    }
}

fn make_u16(low: u8, high: u8) -> u16 {
    (high as u16) << 8 | low as u16
}

fn make_i8(v: u8) -> i8 {
    unsafe { mem::transmute(v) }
}

fn make_cond(byte: u8) -> ConditionType {
    let v = (byte >> 3) & 0b111;
    match v {
        0 => ConditionType::NonZero,
        1 => ConditionType::Zero,
        2 => ConditionType::NotCarry,
        3 => ConditionType::Carry,
        _ => panic!("Unknown condition code {}!", v),
    }
}

fn make_operand_from_r8(byte: u8, lower_3_bits: bool) -> Operand {
    let v: u8;
    if lower_3_bits {
        v = byte & 0b111;
    } else {
        v = (byte >> 3) & 0b111;
    }
    match v {
        0 => Operand::Register(Register::B),
        1 => Operand::Register(Register::C),
        2 => Operand::Register(Register::D),
        3 => Operand::Register(Register::E),
        4 => Operand::Register(Register::H),
        5 => Operand::Register(Register::L),
        6 => Operand::Deref(DerefOperand::Register(Register::Hl)),
        7 => Operand::Register(Register::A),
        _ => panic!("Unknown r8 {}!", v),
    }
}

fn make_r16(byte: u8, group: u8) -> Register {
    let v = (byte >> 4) & 0b11;
    assert!(group < 4);
    assert!(v < 4);
    let r = match v {
        0 => Register::Bc,
        1 => Register::De,
        2 => match group {
            1 | 3 => Register::Hl,
            2 => Register::HlPlus,
            _ => unreachable!(),
        },
        3 => match group {
            1 => Register::Sp,
            2 => Register::HlMinus,
            3 => Register::Af,
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };

    r
}

fn get_subopcode(byte: u8) -> u8 {
    (byte >> 3) & 0b111
}

fn get_exp(byte: u8) -> u8 {
    let v = (byte >> 3) & 0b11;
    v * 8
}

fn get_bit(byte: u8) -> u8 {
    (byte >> 3) & 0b111
}

// Helper functions for instruction decoding
/// Match given byte against opcode, masking out
/// the condition
fn match_cond_mask(byte: u8, opcode: u8) -> bool {
    (byte & 0b11100111) == opcode
}

/// Match given byte against opcode, masking out
/// the r16
fn match_r16_mask(byte: u8, opcode: u8) -> bool {
    (byte & 0b11001111) == opcode
}

/// Match given byte against opcode, masking out
/// the r8
fn match_r8_mask(byte: u8, opcode: u8) -> bool {
    (byte & 0b11000111) == opcode
}

/// Match given byte against opcode, masking out
/// the two r8s
fn match_two_r8_mask(byte: u8, opcode: u8) -> bool {
    (byte & 0b11000000) == opcode
}

/// Match given byte against opcode, masking out
/// the subopcode and r8
fn match_subopcode_r8_mask(byte: u8, opcode: u8) -> bool {
    (byte & 0b11000000) == opcode
}

/// Match given byte against opcode, masking out
/// the subopcode
fn match_subopcode_mask(byte: u8, opcode: u8) -> bool {
    (byte & 0b11000111) == opcode
}

/// Match given byte against opcode, masking out
/// the exp
fn match_exp_mask(byte: u8, opcode: u8) -> bool {
    (byte & 0b11000111) == opcode
}

#[derive(Debug)]
pub struct Instruction {
    pub address: u16,
    pub op: Opcode,
    pub size: u8,
}

impl Instruction {
    pub fn new(address: u16, raw_bytes: &[u8]) -> Self {
        let byte = raw_bytes[0];

        // Check for 0xcb prefix instructions
        if byte == 0xcb {
            let byte = raw_bytes[1];
            return match byte >> 6 {
                0 => {
                    // Opcode (group 3)
                    let register = make_operand_from_r8(byte, true);
                    match get_subopcode(byte) {
                        0 => Instruction {
                            address,
                            op: Opcode::Rlc { operand: register },
                            size: 2,
                        },
                        1 => Instruction {
                            address,
                            op: Opcode::Rrc { operand: register },
                            size: 2,
                        },
                        2 => Instruction {
                            address,
                            op: Opcode::Rl { operand: register },
                            size: 2,
                        },
                        3 => Instruction {
                            address,
                            op: Opcode::Rr { operand: register },
                            size: 2,
                        },
                        4 => Instruction {
                            address,
                            op: Opcode::Sla { operand: register },
                            size: 2,
                        },
                        5 => Instruction {
                            address,
                            op: Opcode::Sra { operand: register },
                            size: 2,
                        },
                        6 => Instruction {
                            address,
                            op: Opcode::Swap { operand: register },
                            size: 2,
                        },
                        7 => Instruction {
                            address,
                            op: Opcode::Srl { operand: register },
                            size: 2,
                        },
                        _ => unreachable!(),
                    }
                }
                1 => Instruction {
                    address,
                    op: Opcode::Bit {
                        bit: get_bit(byte),
                        destination: make_operand_from_r8(byte, true),
                    },
                    size: 2,
                },
                2 => Instruction {
                    address,
                    op: Opcode::Res {
                        bit: get_bit(byte),
                        destination: make_operand_from_r8(byte, true),
                    },
                    size: 2,
                },
                3 => Instruction {
                    address,
                    op: Opcode::Set {
                        bit: get_bit(byte),
                        destination: make_operand_from_r8(byte, true),
                    },
                    size: 2,
                },
                _ => unreachable!(),
            };
        }

        // match the simple opcodes
        match byte {
            0 => {
                return Instruction {
                    address,
                    op: Opcode::Nop,
                    size: 1,
                }
            }
            8 => {
                return Instruction {
                    address,
                    op: Opcode::Ld16 {
                        destination: Operand::Deref(DerefOperand::Address(make_u16(
                            raw_bytes[1],
                            raw_bytes[2],
                        ))),
                        source: Operand::Register(Register::Sp),
                    },
                    size: 3,
                }
            }
            16 => {
                return Instruction {
                    address,
                    op: Opcode::Stop,
                    size: 1,
                }
            }
            24 => {
                return Instruction {
                    address,
                    op: Opcode::Jr {
                        offset: make_i8(raw_bytes[1]),
                    },
                    size: 2,
                }
            }
            118 => {
                return Instruction {
                    address,
                    op: Opcode::Halt,
                    size: 1,
                }
            }
            205 => {
                return Instruction {
                    address,
                    op: Opcode::Call {
                        destination: make_u16(raw_bytes[1], raw_bytes[2]),
                    },
                    size: 3,
                }
            }
            224 => {
                return Instruction {
                    address,
                    op: Opcode::Ld8 {
                        destination: Operand::Deref(DerefOperand::Ff00Offset(raw_bytes[1])),
                        source: Operand::Register(Register::A),
                    },
                    size: 2,
                }
            }
            226 => {
                return Instruction {
                    address,
                    op: Opcode::Ld8 {
                        destination: Operand::Deref(DerefOperand::Ff00PlusC),
                        source: Operand::Register(Register::A),
                    },
                    size: 1,
                }
            }
            232 => {
                return Instruction {
                    address,
                    op: Opcode::Add16 {
                        register: Register::Sp,
                        operand: Operand::I8(make_i8(raw_bytes[1])),
                    },
                    size: 2,
                }
            }
            234 => {
                return Instruction {
                    address,
                    op: Opcode::Ld8 {
                        destination: Operand::Deref(DerefOperand::Address(make_u16(
                            raw_bytes[1],
                            raw_bytes[2],
                        ))),
                        source: Operand::Register(Register::A),
                    },
                    size: 3,
                }
            }
            240 => {
                return Instruction {
                    address,
                    op: Opcode::Ld8 {
                        destination: Operand::Register(Register::A),
                        source: Operand::Deref(DerefOperand::Ff00Offset(raw_bytes[1])),
                    },
                    size: 2,
                }
            }
            242 => {
                return Instruction {
                    address,
                    op: Opcode::Ld8 {
                        destination: Operand::Register(Register::A),
                        source: Operand::Deref(DerefOperand::Ff00PlusC),
                    },
                    size: 1,
                }
            }
            248 => {
                return Instruction {
                    address,
                    op: Opcode::Ld16 {
                        destination: Operand::Register(Register::Hl),
                        source: Operand::StackOffset(make_i8(raw_bytes[1])),
                    },
                    size: 2,
                }
            }
            250 => {
                return Instruction {
                    address,
                    op: Opcode::Ld8 {
                        destination: Operand::Register(Register::A),
                        source: Operand::Deref(DerefOperand::Address(make_u16(
                            raw_bytes[1],
                            raw_bytes[2],
                        ))),
                    },
                    size: 3,
                }
            }
            _ => (),
        }

        // Now decode more complicated instructions
        if match_cond_mask(byte, 0b00100000) {
            return Instruction {
                address,
                op: Opcode::JrCond {
                    condition: make_cond(byte),
                    offset: make_i8(raw_bytes[1]),
                },
                size: 2,
            };
        } else if match_r16_mask(byte, 0b00000001) {
            return Instruction {
                address,
                op: Opcode::Ld16 {
                    destination: Operand::Register(make_r16(byte, 1)),
                    source: Operand::U16(make_u16(raw_bytes[1], raw_bytes[2])),
                },
                size: 3,
            };
        } else if match_r16_mask(byte, 0b00000010) {
            return Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Deref(DerefOperand::Register(make_r16(byte, 2))),
                    source: Operand::Register(Register::A),
                },
                size: 1,
            };
        } else if match_r16_mask(byte, 0b00000011) {
            return Instruction {
                address,
                op: Opcode::Inc16 {
                    register: make_r16(byte, 1),
                },
                size: 1,
            };
        } else if match_r16_mask(byte, 0b00001001) {
            return Instruction {
                address,
                op: Opcode::Add16 {
                    register: Register::Hl,
                    operand: Operand::Register(make_r16(byte, 1)),
                },
                size: 1,
            };
        } else if match_r16_mask(byte, 0b00001010) {
            return Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: Operand::Register(Register::A),
                    source: Operand::Deref(DerefOperand::Register(make_r16(byte, 2))),
                },
                size: 1,
            };
        } else if match_r16_mask(byte, 0b00001011) {
            return Instruction {
                address,
                op: Opcode::Dec16 {
                    register: make_r16(byte, 1),
                },
                size: 1,
            };
        } else if match_r8_mask(byte, 0b00000100) {
            return Instruction {
                address,
                op: Opcode::Inc {
                    operand: make_operand_from_r8(byte, false),
                },
                size: 1,
            };
        } else if match_r8_mask(byte, 0b00000101) {
            return Instruction {
                address,
                op: Opcode::Dec {
                    operand: make_operand_from_r8(byte, false),
                },
                size: 1,
            };
        } else if match_r8_mask(byte, 0b00000110) {
            return Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: make_operand_from_r8(byte, false),
                    source: Operand::U8(raw_bytes[1]),
                },
                size: 2,
            };
        } else if match_subopcode_mask(byte, 0b00000111) {
            // Opcode Group 1
            return match get_subopcode(byte) {
                0 => Instruction {
                    address,
                    op: Opcode::Rlca,
                    size: 1,
                },
                1 => Instruction {
                    address,
                    op: Opcode::Rrca,
                    size: 1,
                },
                2 => Instruction {
                    address,
                    op: Opcode::Rla,
                    size: 1,
                },
                3 => Instruction {
                    address,
                    op: Opcode::Rra,
                    size: 1,
                },
                4 => Instruction {
                    address,
                    op: Opcode::Daa,
                    size: 1,
                },
                5 => Instruction {
                    address,
                    op: Opcode::Cpl,
                    size: 1,
                },
                6 => Instruction {
                    address,
                    op: Opcode::Scf,
                    size: 1,
                },
                7 => Instruction {
                    address,
                    op: Opcode::Ccf,
                    size: 1,
                },
                _ => unreachable!(),
            };
        } else if match_two_r8_mask(byte, 0b01000000) {
            return Instruction {
                address,
                op: Opcode::Ld8 {
                    destination: make_operand_from_r8(byte, false),
                    source: make_operand_from_r8(byte, true),
                },
                size: 1,
            };
        } else if match_subopcode_r8_mask(byte, 0b10000000) {
            // Opcode (group 3)
            let operand = make_operand_from_r8(byte, true);
            return match get_subopcode(byte) {
                0 => Instruction {
                    address,
                    op: Opcode::Add8 { operand },
                    size: 1,
                },
                1 => Instruction {
                    address,
                    op: Opcode::Adc { operand },
                    size: 1,
                },
                2 => Instruction {
                    address,
                    op: Opcode::Sub { operand },
                    size: 1,
                },
                3 => Instruction {
                    address,
                    op: Opcode::Sbc { operand },
                    size: 1,
                },
                4 => Instruction {
                    address,
                    op: Opcode::And { operand },
                    size: 1,
                },
                5 => Instruction {
                    address,
                    op: Opcode::Xor { operand },
                    size: 1,
                },
                6 => Instruction {
                    address,
                    op: Opcode::Or { operand },
                    size: 1,
                },
                7 => Instruction {
                    address,
                    op: Opcode::Cp { operand },
                    size: 1,
                },
                _ => unreachable!(),
            };
        } else if match_cond_mask(byte, 0b11000000) {
            return Instruction {
                address,
                op: Opcode::RetCond {
                    condition: make_cond(byte),
                },
                size: 1,
            };
        } else if match_cond_mask(byte, 0b11000010) {
            return Instruction {
                address,
                op: Opcode::JpCond {
                    condition: make_cond(byte),
                    destination: make_u16(raw_bytes[1], raw_bytes[2]),
                },
                size: 3,
            };
        } else if match_cond_mask(byte, 0b11000100) {
            return Instruction {
                address,
                op: Opcode::CallCond {
                    condition: make_cond(byte),
                    destination: make_u16(raw_bytes[1], raw_bytes[2]),
                },
                size: 3,
            };
        } else if match_r16_mask(byte, 0b11000001) {
            return Instruction {
                address,
                op: Opcode::Pop {
                    register: make_r16(byte, 3),
                },
                size: 1,
            };
        } else if (byte & 0b00110000) == 0b11001001 {
            let opcode = (byte >> 4) & 0b11;
            return match opcode {
                0 => Instruction {
                    address,
                    op: Opcode::Ret,
                    size: 1,
                },
                1 => Instruction {
                    address,
                    op: Opcode::Reti,
                    size: 1,
                },
                2 => Instruction {
                    address,
                    op: Opcode::Jp {
                        destination: Operand::Register(Register::Hl),
                    },
                    size: 1,
                },
                3 => Instruction {
                    address,
                    op: Opcode::Ld16 {
                        destination: Operand::Register(Register::Sp),
                        source: Operand::Register(Register::Hl),
                    },
                    size: 1,
                },
                _ => unreachable!(),
            };
        } else if match_subopcode_mask(byte, 0b11000011) {
            // opcode (group 4)
            return match get_subopcode(byte) {
                0 => Instruction {
                    address,
                    op: Opcode::Jp {
                        destination: Operand::U16(make_u16(raw_bytes[1], raw_bytes[2])),
                    },
                    size: 3,
                },
                1 => unreachable!(), // technically this should be the CB prefix but we checked for that earlier
                6 => Instruction {
                    address,
                    op: Opcode::Di,
                    size: 1,
                },
                7 => Instruction {
                    address,
                    op: Opcode::Ei,
                    size: 1,
                },
                _ => Instruction {
                    address,
                    op: Opcode::Unknown,
                    size: 1,
                },
            };
        } else if match_r16_mask(byte, 0b11000101) {
            return Instruction {
                address,
                op: Opcode::Push {
                    register: make_r16(byte, 3),
                },
                size: 1,
            };
        } else if match_subopcode_mask(byte, 0b11000110) {
            let operand = Operand::U8(raw_bytes[1]);
            return match get_subopcode(byte) {
                0 => Instruction {
                    address,
                    op: Opcode::Add8 { operand },
                    size: 1,
                },
                1 => Instruction {
                    address,
                    op: Opcode::Adc { operand },
                    size: 1,
                },
                2 => Instruction {
                    address,
                    op: Opcode::Sub { operand },
                    size: 1,
                },
                3 => Instruction {
                    address,
                    op: Opcode::Sbc { operand },
                    size: 1,
                },
                4 => Instruction {
                    address,
                    op: Opcode::And { operand },
                    size: 1,
                },
                5 => Instruction {
                    address,
                    op: Opcode::Xor { operand },
                    size: 1,
                },
                6 => Instruction {
                    address,
                    op: Opcode::Or { operand },
                    size: 1,
                },
                7 => Instruction {
                    address,
                    op: Opcode::Cp { operand },
                    size: 1,
                },
                _ => unreachable!(),
            };
        } else if match_exp_mask(byte, 0b11000111) {
            return Instruction {
                address,
                op: Opcode::Rst {
                    vector: get_exp(byte),
                },
                size: 1,
            };
        }

        //eprintln!("Unknown op code {:02x}", byte);
        Instruction {
            address,
            op: Opcode::Unknown,
            size: 1,
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "0x{:04x} - {} (size = {})",
            self.address, self.op, self.size
        )
    }
}
