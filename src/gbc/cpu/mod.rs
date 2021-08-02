pub mod instruction;
pub mod register;

use register::RegisterStorage;

use crate::gbc::cpu::instruction::{
    ConditionType, DerefOperand, Instruction, Opcode, Operand, Register,
};

use crate::gbc::memory_bus::MemoryBus;

// Flags register bits
const CARRY_BIT_MASK: u8 = 1 << 4;
const HALF_CARRY_BIT_MASK: u8 = 1 << 5;
const SUBTRACTION_BIT_MASK: u8 = 1 << 6;
const ZERO_BIT_MASK: u8 = 1 << 7;

#[derive(Debug)]
pub enum CpuState {
    Running,
    Halted,
    Stopped,
}

#[derive(Debug)]
pub struct Cpu {
    af: RegisterStorage,
    bc: RegisterStorage,
    de: RegisterStorage,
    hl: RegisterStorage,
    pc: u16,
    sp: u16,
    state: CpuState,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu {
            af: RegisterStorage::default(),
            bc: RegisterStorage::default(),
            de: RegisterStorage::default(),
            hl: RegisterStorage::default(),
            pc: 0x100,
            sp: 0,
            state: CpuState::Running,
        }
    }
}

const INTERRUPT_ENABLE_REGISTER_ADDRESS: u16 = 0xffff;
const INTERRUPT_FLAGS_REGISTER_ADDRESS: u16 = 0xff0f;

impl Cpu {
    pub fn interrupt(&mut self, memory_bus: &mut MemoryBus, interrupt_number: u8) {
        assert!(interrupt_number < 5);
        let mut interrupt_flags = memory_bus.read_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS);
        Self::set_bit(interrupt_number, &mut interrupt_flags);
        memory_bus.write_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS, interrupt_flags);
    }

    pub fn get_program_counter(&self) -> u16 {
        self.pc
    }

    pub fn get_instruction_at_address(&self, memory_bus: &MemoryBus, address: u16) -> Instruction {
        let insn_bytes = memory_bus.read_mem(address, 3);
        Instruction::new(address, &insn_bytes[..])
    }

    pub fn get_next_instruction(&self, memory_bus: &MemoryBus) -> Instruction {
        self.get_instruction_at_address(memory_bus, self.pc)
    }

    pub fn single_step(&mut self, memory_bus: &mut MemoryBus) -> u64 {
        if self.should_service_interrupt(memory_bus) {
            self.service_interrupt(memory_bus);
            return 5;
        }
        let insn = self.get_next_instruction(memory_bus);
        //println!("{}", insn);

        self.pc += insn.size as u16;

        match insn.op {
            Opcode::Unknown => panic!("Unknown instruction!"),
            Opcode::Nop => 4,
            Opcode::Stop => {
                self.state = CpuState::Stopped;
                4
            }
            Opcode::Halt => {
                self.state = CpuState::Halted;
                4
            }
            Opcode::Ld8 {
                destination,
                source,
            } => match destination {
                Operand::Register(r_dest) => match source {
                    Operand::Register(r_src) => {
                        let v = self.get_r8(&r_src);
                        self.set_r8(&r_dest, v);
                        4
                    }
                    Operand::U8(v) => {
                        self.set_r8(&r_dest, v);
                        8
                    }
                    Operand::Deref(d) => match d {
                        DerefOperand::Register(Register::Hl) => {
                            let v = memory_bus.read_u8(self.hl.get_u16());
                            self.set_r8(&r_dest, v);
                            8
                        }
                        DerefOperand::Register(r_src)
                            if r_src == Register::Bc || r_src == Register::De =>
                        {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (BC) or (DE)!");
                            }
                            let v = memory_bus.read_u8(self.get_r16(&r_src));
                            self.set_a(v);
                            8
                        }
                        DerefOperand::Register(Register::HlPlus) => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (Hl+)!");
                            }
                            let v = {
                                let hl = self.hl.get_u16_mut();
                                let temp = memory_bus.read_u8(*hl);
                                *hl += 1;
                                temp
                            };
                            self.set_a(v);
                            8
                        }
                        DerefOperand::Register(Register::HlMinus) => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (Hl-)!");
                            }
                            let v = {
                                let hl = self.hl.get_u16_mut();
                                let temp = memory_bus.read_u8(*hl);
                                *hl -= 1;
                                temp
                            };
                            self.set_a(v);
                            8
                        }
                        DerefOperand::Address(addr) => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (nn)!");
                            }
                            let v = memory_bus.read_u8(addr);
                            self.set_a(v);
                            16
                        }
                        DerefOperand::Ff00Offset(offset) => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (0xff00+n)!");
                            }
                            let v = memory_bus.read_u8(0xff00u16.wrapping_add(offset as u16));
                            self.set_a(v);
                            12
                        }
                        DerefOperand::Ff00PlusC => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (0xff00+C)!");
                            }
                            let v = memory_bus
                                .read_u8(0xff00u16.wrapping_add(self.bc.get_low() as u16));
                            self.set_a(v);
                            8
                        }
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                },
                Operand::Deref(d) => match d {
                    DerefOperand::Register(r) => match r {
                        Register::Hl => {
                            let (v, cycles) = match source {
                                Operand::Register(r) => (self.get_r8(&r), 8),
                                Operand::U8(v) => (v, 12),
                                _ => unreachable!(),
                            };
                            memory_bus.write_u8(self.hl.get_u16(), v);
                            cycles
                        }
                        Register::Bc | Register::De => {
                            if source != Operand::Register(Register::A) {
                                panic!("Source must be A for load to (BC) or (DE)!");
                            }
                            let v = self.get_a();
                            memory_bus.write_u8(self.get_r16(&r), v);
                            8
                        }
                        Register::HlPlus => {
                            if source != Operand::Register(Register::A) {
                                panic!("Source must be A for load to (Hl+)!");
                            }
                            let v = self.get_a();
                            let hl = self.hl.get_u16_mut();
                            memory_bus.write_u8(*hl, v);
                            *hl += 1;
                            8
                        }
                        Register::HlMinus => {
                            if source != Operand::Register(Register::A) {
                                panic!("Source must be A for load to (Hl-)!");
                            }
                            let v = self.get_a();
                            let hl = self.hl.get_u16_mut();
                            memory_bus.write_u8(*hl, v);
                            *hl -= 1;
                            8
                        }
                        _ => unreachable!(),
                    },
                    DerefOperand::Address(addr) => {
                        if source != Operand::Register(Register::A) {
                            panic!("Source must be A for load to (nn)!");
                        }
                        let v = self.get_a();
                        memory_bus.write_u8(addr, v);
                        16
                    }
                    DerefOperand::Ff00Offset(offset) => {
                        if source != Operand::Register(Register::A) {
                            panic!("Source must be A for load to (0xff00+n)!");
                        }
                        let v = self.get_a();
                        memory_bus.write_u8(0xff00u16.wrapping_add(offset as u16), v);
                        8
                    }
                    DerefOperand::Ff00PlusC => {
                        if source != Operand::Register(Register::A) {
                            panic!("Source must be A for load to (0xff00+C)!");
                        }
                        let v = self.get_a();
                        memory_bus.write_u8(0xff00u16.wrapping_add(self.bc.get_low() as u16), v);
                        8
                    }
                },
                _ => unreachable!(),
            },
            Opcode::Ld16 {
                destination,
                source,
            } => match destination {
                Operand::Register(r)
                    if r == Register::Bc
                        || r == Register::De
                        || r == Register::Hl
                        || r == Register::Sp =>
                {
                    match source {
                        Operand::U16(v) => {
                            self.set_r16(&r, v);
                            12
                        }
                        Operand::Register(Register::Hl) => {
                            if r != Register::Sp {
                                panic!("Destination must be SP for ld from HL!");
                            }
                            self.set_r16(&r, self.hl.get_u16());
                            8
                        }
                        _ => unreachable!(),
                    }
                }
                Operand::Deref(DerefOperand::Address(addr)) => match source {
                    Operand::Register(Register::Sp) => {
                        memory_bus.write_u16(addr, self.sp);
                        20
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            Opcode::Jp { destination } => match destination {
                Operand::U16(address) => {
                    self.pc = address;
                    16
                }
                Operand::Register(Register::Hl) => {
                    self.pc = self.hl.get_u16();
                    4
                }
                _ => unreachable!(),
            },
            Opcode::JpCond {
                condition,
                destination,
            } => {
                if self.check_condition(condition) {
                    self.pc = destination;
                    16
                } else {
                    12
                }
            }
            Opcode::Jr { offset } => {
                self.pc = (self.pc as i32 + offset as i32) as u16;
                12
            }
            Opcode::JrCond { condition, offset } => {
                if self.check_condition(condition) {
                    self.pc = (self.pc as i32 + offset as i32) as u16;
                    12
                } else {
                    8
                }
            }
            Opcode::Call { destination } => {
                self.call(memory_bus, destination);
                24
            }
            Opcode::CallCond {
                condition,
                destination,
            } => {
                if self.check_condition(condition) {
                    self.call(memory_bus, destination);
                    24
                } else {
                    16
                }
            }
            Opcode::Ret => {
                self.ret(memory_bus);
                16
            }
            Opcode::RetCond { condition } => {
                if self.check_condition(condition) {
                    self.ret(memory_bus);
                    20
                } else {
                    8
                }
            }
            Opcode::Reti => {
                self.ret(memory_bus);
                self.enable_interrupts(memory_bus);
                16
            }
            Opcode::Pop { register } => {
                match register {
                    Register::Bc => {
                        let v = self.pop(memory_bus);
                        self.bc.set_u16(v);
                    }
                    Register::De => {
                        let v = self.pop(memory_bus);
                        self.de.set_u16(v);
                    }
                    Register::Hl => {
                        let v = self.pop(memory_bus);
                        self.hl.set_u16(v);
                    }
                    Register::Af => {
                        let v = self.pop(memory_bus);
                        self.af.set_u16(v);
                    }
                    _ => unreachable!(),
                }
                12
            }
            Opcode::Push { register } => {
                match register {
                    Register::Bc => self.push(memory_bus, self.bc.get_u16()),
                    Register::De => self.push(memory_bus, self.de.get_u16()),
                    Register::Hl => self.push(memory_bus, self.hl.get_u16()),
                    Register::Af => self.push(memory_bus, self.af.get_u16()),
                    _ => unreachable!(),
                }
                16
            }
            Opcode::Rst { vector } => {
                self.call(memory_bus, vector as u16);
                16
            }
            Opcode::Bit { bit, destination } => {
                // Clear subtraction flag, set half-carry flag
                self.clear_subtraction_flag();
                self.set_half_carry_flag();
                match destination {
                    Operand::Register(r) => {
                        let v = self.get_r8(&r);
                        if Self::test_bit(bit, v) {
                            self.set_zero_flag();
                        } else {
                            self.clear_zero_flag();
                        }
                        8
                    }
                    Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                        let v = memory_bus.read_u8(self.hl.get_u16());
                        if Self::test_bit(bit, v) {
                            self.set_zero_flag();
                        } else {
                            self.clear_zero_flag();
                        }
                        12
                    }
                    _ => unreachable!(),
                }
            }
            Opcode::Res { bit, destination } => match destination {
                Operand::Register(r) => {
                    let v = self.get_r8_mut(&r);
                    Self::reset_bit(bit, v);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    Self::reset_bit(bit, &mut v);
                    memory_bus.write_u8(self.hl.get_u16(), v);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Set { bit, destination } => match destination {
                Operand::Register(r) => {
                    let v = self.get_r8_mut(&r);
                    Self::set_bit(bit, v);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    Self::set_bit(bit, &mut v);
                    memory_bus.write_u8(self.hl.get_u16(), v);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Add8 { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                self.add8_with_carry(v, false, false);
                cycles
            }
            Opcode::Add16 { register, operand } => {
                match register {
                    Register::Hl => {
                        if let Operand::Register(r) = operand {
                            let v = match r {
                                Register::Bc => self.bc.get_u16(),
                                Register::De => self.de.get_u16(),
                                Register::Hl => self.hl.get_u16(),
                                Register::Sp => self.sp,
                                _ => unreachable!(),
                            };
                            let hl = self.hl.get_u16();
                            let (sum, carry) = hl.overflowing_add(v);
                            let zero = self.get_zero_flag();
                            let subtraction = false;
                            let half_carry = (hl & 0xfff) + (v & 0xfff) > 0xfff;
                            self.hl.set_u16(sum);
                            self.set_flags_from_bools(zero, subtraction, half_carry, carry);
                            8
                        } else {
                            unreachable!()
                        }
                    }
                    Register::Sp => {
                        if let Operand::I8(d) = operand {
                            // Convert the i8 to a u16 and use overflowing add
                            let d_u8 = d.to_le_bytes()[0];
                            let d_u16 = u16::from_le_bytes([d_u8, 0xFF]);
                            let (sum, carry) = self.sp.overflowing_add(d_u16);
                            let zero = false;
                            let subtraction = false;
                            let half_carry = (self.sp & 0xfff) + (d_u16 & 0xfff) > 0xfff;
                            self.sp = sum;
                            self.set_flags_from_bools(zero, subtraction, half_carry, carry);
                            16
                        } else {
                            unreachable!()
                        }
                    }
                    _ => unreachable!(),
                }
            }
            Opcode::Inc { operand } => match operand {
                Operand::Register(r) => {
                    let v = self.get_r8(&r);
                    let (res, _) = v.overflowing_add(1);
                    self.set_r8(&r, res);
                    let zero = res == 0;
                    let subtraction = false;
                    let half_carry = (res & 0xf) == 0xf;
                    let carry = self.get_carry_flag();
                    self.set_flags_from_bools(zero, subtraction, half_carry, carry);
                    4
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let v = memory_bus.read_u8(self.hl.get_u16());
                    let (res, _) = v.overflowing_add(1);
                    memory_bus.write_u8(self.hl.get_u16(), res);
                    let zero = res == 0;
                    let subtraction = false;
                    let half_carry = (res & 0xf) == 0xf;
                    let carry = self.get_carry_flag();
                    self.set_flags_from_bools(zero, subtraction, half_carry, carry);
                    12
                }
                _ => unreachable!(),
            },
            Opcode::Inc16 { register } => {
                match register {
                    Register::Bc => {
                        let reg = self.bc.get_u16_mut();
                        *reg = reg.wrapping_add(1);
                    }
                    Register::De => {
                        let reg = self.de.get_u16_mut();
                        *reg = reg.wrapping_add(1);
                    }
                    Register::Hl => {
                        let reg = self.hl.get_u16_mut();
                        *reg = reg.wrapping_add(1);
                    }
                    Register::Sp => {
                        let reg = &mut self.sp;
                        *reg = reg.wrapping_add(1);
                    }
                    _ => unreachable!(),
                };
                8
            }
            Opcode::Dec { operand } => match operand {
                Operand::Register(r) => {
                    let v = self.get_r8(&r);
                    let (res, _) = v.overflowing_add(0xff); // - 1 is the same as + 0xff
                    self.set_r8(&r, res);
                    let zero = res == 0;
                    let subtraction = false;
                    let half_carry = (res & 0xf) == 0xf;
                    let carry = self.get_carry_flag();
                    self.set_flags_from_bools(zero, subtraction, half_carry, carry);
                    4
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let v = memory_bus.read_u8(self.hl.get_u16());
                    let (res, _) = v.overflowing_add(0xff); // - 1 is the same as + 0xff
                    memory_bus.write_u8(self.hl.get_u16(), res);
                    let zero = res == 0;
                    let subtraction = false;
                    let half_carry = (res & 0xf) == 0xf;
                    let carry = self.get_carry_flag();
                    self.set_flags_from_bools(zero, subtraction, half_carry, carry);
                    12
                }
                _ => unreachable!(),
            },
            Opcode::Dec16 { register } => {
                match register {
                    Register::Bc => {
                        let reg = self.bc.get_u16_mut();
                        *reg = reg.wrapping_sub(1);
                    }
                    Register::De => {
                        let reg = self.de.get_u16_mut();
                        *reg = reg.wrapping_sub(1);
                    }
                    Register::Hl => {
                        let reg = self.hl.get_u16_mut();
                        *reg = reg.wrapping_sub(1);
                    }
                    Register::Sp => {
                        let reg = &mut self.sp;
                        *reg = reg.wrapping_sub(1);
                    }
                    _ => unreachable!(),
                };
                8
            }
            Opcode::Adc { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                self.add8_with_carry(v, true, false);
                cycles
            }
            Opcode::Sub { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                self.add8_with_carry((!v).wrapping_add(1), false, true);
                cycles
            }
            Opcode::Sbc { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                self.add8_with_carry(!v + 1, true, true);
                cycles
            }
            Opcode::And { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                let val = {
                    let a = self.get_a_mut();
                    *a = *a & v;
                    *a
                };
                self.set_flags_from_bools(val == 0, false, true, false);
                cycles
            }
            Opcode::Xor { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                let val = {
                    let a = self.get_a_mut();
                    *a = *a & v;
                    *a
                };
                self.set_flags_from_bools(val == 0, false, false, false);
                cycles
            }
            Opcode::Or { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                let val = {
                    let a = self.get_a_mut();
                    *a = *a & v;
                    *a
                };
                self.set_flags_from_bools(val == 0, false, false, false);
                cycles
            }
            Opcode::Cp { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);
                // Compute A - v, but only set flags according to the result
                let a = self.get_a();
                let is_zero = a == v;
                let is_half_carry = (a % 16) < (v % 16);
                let is_carry = a < v;
                let is_subtraction = true;

                self.set_flags_from_bools(is_zero, is_subtraction, is_half_carry, is_carry);
                cycles
            }
            Opcode::Cpl => {
                self.set_a(!self.get_a());
                4
            }
            Opcode::Daa => {
                // from https://forums.nesdev.com/viewtopic.php?t=15944
                // If not subtraction
                let mut a = self.get_a();
                let mut carry = self.get_carry_flag();
                if self.get_subtraction_flag() {
                    if self.get_carry_flag() || a > 0x99 {
                        a += 0x60;
                        carry = true;
                    }
                    if self.get_half_carry_flag() || (a & 0xf) > 0x9 {
                        a += 0x6;
                    }
                } else {
                    if self.get_carry_flag() {
                        a -= 0x60;
                    }
                    if self.get_half_carry_flag() {
                        a -= 0x6;
                    }
                }

                let zero = a == 0;
                let half_carry = false;
                let subtraction = self.get_subtraction_flag();
                self.set_a(a);
                self.set_flags_from_bools(zero, subtraction, half_carry, carry);
                4
            }
            Opcode::Rlca => {
                let mut a = self.get_a();
                let high_bit = a >> 7;
                let carry = high_bit == 1;
                a = (a << 1) & high_bit;
                self.set_a(a);

                // only the carry flag should be set after this;
                self.clear_flags();
                self.set_carry_flag_from_bool(carry);
                4
            }
            Opcode::Rla => {
                let mut a = self.get_a();
                let old_carry = self.get_carry_flag() as u8;
                let high_bit = a >> 7;
                let carry = high_bit == 1;
                a = (a << 1) & old_carry;
                self.set_a(a);

                // only the carry flag should be set after this;
                self.clear_flags();
                self.set_carry_flag_from_bool(carry);
                4
            }
            Opcode::Rrca => {
                let mut a = self.get_a();
                let low_bit = a & 1;
                let carry = low_bit == 1;
                a = (a >> 1) & (low_bit << 7);
                self.set_a(a);

                // only the carry flag should be set after this;
                self.clear_flags();
                self.set_carry_flag_from_bool(carry);
                4
            }
            Opcode::Rra => {
                let mut a = self.get_a();
                let old_carry = self.get_carry_flag() as u8;
                let low_bit = a & 1;
                let carry = low_bit == 1;
                a = (a >> 1) & (old_carry << 7);
                self.set_a(a);

                // only the carry flag should be set after this;
                self.clear_flags();
                self.set_carry_flag_from_bool(carry);
                4
            }
            Opcode::Rlc { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = (v << 1) & high_bit;
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = (v << 1) & high_bit;
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Rl { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let old_carry = self.get_carry_flag() as u8;
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = (v << 1) & old_carry;
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let old_carry = self.get_carry_flag() as u8;
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = (v << 1) & old_carry;
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Rrc { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = (v >> 1) & (low_bit << 7);
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = (v >> 1) & (low_bit << 7);
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Rr { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let old_carry = self.get_carry_flag() as u8;
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = (v >> 1) & (old_carry << 7);
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let old_carry = self.get_carry_flag() as u8;
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = (v >> 1) & (old_carry << 7);
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Sla { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v *= 2;
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v *= 2;
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Swap { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    v = (v >> 4) | (v << 4);
                    self.set_r8(&r, v);

                    // only the zero flag should be set after this;
                    self.clear_flags();
                    self.set_zero_flag_from_bool(v == 0);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    v = (v >> 4) | (v << 4);
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the zero flag should be set after this;
                    self.clear_flags();
                    self.set_zero_flag_from_bool(v == 0);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Sra { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = ((v as i8) >> 1) as u8;
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = ((v as i8) >> 1) as u8;
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Srl { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = v >> 1;
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    8
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = v >> 1;
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    16
                }
                _ => unreachable!(),
            },
            Opcode::Scf => {
                // Sets carry to 1, clears half carry and subtraction
                self.set_carry_flag();
                self.clear_half_carry_flag();
                self.clear_subtraction_flag();
                4
            }
            Opcode::Ccf => {
                // Toggles Carry, clears half carry and subtraction
                if self.get_carry_flag() {
                    self.clear_carry_flag();
                } else {
                    self.set_carry_flag();
                }
                self.clear_half_carry_flag();
                self.clear_subtraction_flag();
                4
            }
            Opcode::Di => {
                self.disable_interrupts(memory_bus);
                4
            }
            Opcode::Ei => {
                self.enable_interrupts(memory_bus);
                4
            }
        }
    }

    fn check_condition(&self, condition: ConditionType) -> bool {
        match condition {
            instruction::ConditionType::NonZero => !self.get_zero_flag(),
            instruction::ConditionType::Zero => self.get_zero_flag(),
            instruction::ConditionType::NotCarry => !self.get_carry_flag(),
            instruction::ConditionType::Carry => self.get_carry_flag(),
        }
    }

    fn push(&mut self, memory_bus: &mut MemoryBus, v: u16) {
        memory_bus.write_mem(self.sp, &v.to_le_bytes()[..]);
        self.sp += 2;
    }

    fn pop(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let v = memory_bus.read_mem(self.sp, 2);
        self.sp -= 2;
        assert!(v.len() == 2);
        ((v[1] as u16) << 8) | (v[0] as u16)
    }

    fn call(&mut self, memory_bus: &mut MemoryBus, address: u16) {
        self.push(memory_bus, self.pc);
        self.pc = address;
    }

    fn ret(&mut self, memory_bus: &mut MemoryBus) {
        self.pc = self.pop(memory_bus);
    }

    fn enable_interrupts(&mut self, memory_bus: &mut MemoryBus) {
        memory_bus.write_u8(INTERRUPT_ENABLE_REGISTER_ADDRESS, 1);
    }

    fn disable_interrupts(&mut self, memory_bus: &mut MemoryBus) {
        memory_bus.write_u8(INTERRUPT_ENABLE_REGISTER_ADDRESS, 0);
    }

    fn test_bit(bit: u8, v: u8) -> bool {
        assert!(bit < 8);
        (v & (1 << bit)) != 0
    }

    fn set_bit(bit: u8, v: &mut u8) {
        assert!(bit < 8);
        *v = *v | (1 << bit);
    }

    fn reset_bit(bit: u8, v: &mut u8) {
        assert!(bit < 8);
        *v = *v & !(1 << bit);
    }

    fn get_flags(&self) -> u8 {
        self.af.get_low()
    }

    fn set_flags(&mut self, flags: u8) {
        self.af.set_low(flags);
    }

    fn set_flags_from_bools(
        &mut self,
        zero: bool,
        subtraction: bool,
        half_carry: bool,
        carry: bool,
    ) {
        let mut v = 0u8;
        if zero {
            v |= ZERO_BIT_MASK
        }
        if subtraction {
            v |= SUBTRACTION_BIT_MASK
        }
        if half_carry {
            v |= HALF_CARRY_BIT_MASK
        }
        if carry {
            v |= CARRY_BIT_MASK
        }
        self.set_flags(v);
    }

    fn add8_with_carry(&mut self, b: u8, use_carry: bool, subtraction: bool) {
        let a = self.af.get_high();
        let c = if use_carry && self.get_carry_flag() {
            if subtraction {
                0xFF
            } else {
                1
            }
        } else {
            0
        };
        let (sum, carry) = a.overflowing_add(b);
        let (sum, carry2) = sum.overflowing_add(c);
        let zero = sum == 0;
        let half_carry = ((a & 0xf) + (b & 0xf) + c) > 0xf;
        self.af.set_high(sum as u8);
        self.set_flags_from_bools(zero, subtraction, half_carry, carry || carry2);
    }

    fn extract_u8_arithmetic_operand(
        &self,
        memory_bus: &mut MemoryBus,
        operand: Operand,
    ) -> (u8, u64) {
        match operand {
            Operand::U8(v) => (v, 8),
            Operand::Register(r) => (self.get_r8(&r), 4),
            Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                (memory_bus.read_u8(self.hl.get_u16()), 8)
            }
            _ => unreachable!(),
        }
    }

    fn clear_flags(&mut self) {
        self.set_flags(0);
    }

    fn get_zero_flag(&self) -> bool {
        (self.get_flags() & ZERO_BIT_MASK) == ZERO_BIT_MASK
    }

    fn get_subtraction_flag(&self) -> bool {
        (self.get_flags() & SUBTRACTION_BIT_MASK) == SUBTRACTION_BIT_MASK
    }

    fn get_half_carry_flag(&self) -> bool {
        (self.get_flags() & HALF_CARRY_BIT_MASK) == HALF_CARRY_BIT_MASK
    }

    fn get_carry_flag(&self) -> bool {
        (self.get_flags() & CARRY_BIT_MASK) == CARRY_BIT_MASK
    }

    fn set_zero_flag(&mut self) {
        self.set_flags(self.get_flags() | ZERO_BIT_MASK);
    }

    #[allow(dead_code)]
    fn set_subtraction_flag(&mut self) {
        self.set_flags(self.get_flags() | SUBTRACTION_BIT_MASK);
    }

    fn set_half_carry_flag(&mut self) {
        self.set_flags(self.get_flags() | HALF_CARRY_BIT_MASK);
    }

    fn set_carry_flag(&mut self) {
        self.set_flags(self.get_flags() | CARRY_BIT_MASK);
    }

    fn clear_zero_flag(&mut self) {
        self.set_flags(self.get_flags() & !ZERO_BIT_MASK);
    }

    fn clear_subtraction_flag(&mut self) {
        self.set_flags(self.get_flags() & !SUBTRACTION_BIT_MASK);
    }

    fn clear_half_carry_flag(&mut self) {
        self.set_flags(self.get_flags() & !HALF_CARRY_BIT_MASK);
    }

    fn clear_carry_flag(&mut self) {
        self.set_flags(self.get_flags() & !CARRY_BIT_MASK);
    }

    fn set_zero_flag_from_bool(&mut self, zero: bool) {
        if zero {
            self.set_zero_flag()
        } else {
            self.clear_zero_flag()
        }
    }

    #[allow(dead_code)]
    fn set_subtraction_flag_from_bool(&mut self, subtraction: bool) {
        if subtraction {
            self.set_subtraction_flag()
        } else {
            self.clear_subtraction_flag()
        }
    }

    #[allow(dead_code)]
    fn set_half_carry_flag_from_bool(&mut self, half_carry: bool) {
        if half_carry {
            self.set_half_carry_flag()
        } else {
            self.clear_half_carry_flag()
        }
    }

    fn set_carry_flag_from_bool(&mut self, carry: bool) {
        if carry {
            self.set_carry_flag()
        } else {
            self.clear_carry_flag()
        }
    }

    fn get_a(&self) -> u8 {
        self.af.get_high()
    }

    fn get_a_mut(&mut self) -> &mut u8 {
        self.af.get_high_mut()
    }

    fn set_a(&mut self, v: u8) {
        self.af.set_high(v);
    }

    fn get_r8(&self, register: &Register) -> u8 {
        match register {
            Register::A => self.af.get_high(),
            Register::B => self.bc.get_high(),
            Register::C => self.bc.get_low(),
            Register::D => self.de.get_high(),
            Register::E => self.de.get_low(),
            Register::H => self.hl.get_high(),
            Register::L => self.hl.get_low(),
            _ => unreachable!(), // other registers are not 8 bits
        }
    }

    fn get_r8_mut(&mut self, register: &Register) -> &mut u8 {
        match register {
            Register::A => self.af.get_high_mut(),
            Register::B => self.bc.get_high_mut(),
            Register::C => self.bc.get_low_mut(),
            Register::D => self.de.get_high_mut(),
            Register::E => self.de.get_low_mut(),
            Register::H => self.hl.get_high_mut(),
            Register::L => self.hl.get_low_mut(),
            _ => unreachable!(), // other registers are not 8 bits
        }
    }

    fn set_r8(&mut self, register: &Register, v: u8) {
        match register {
            Register::A => self.af.set_high(v),
            Register::B => self.bc.set_high(v),
            Register::C => self.bc.set_low(v),
            Register::D => self.de.set_high(v),
            Register::E => self.de.set_low(v),
            Register::H => self.hl.set_high(v),
            Register::L => self.hl.set_low(v),
            _ => unreachable!(), // other registers are not 8 bits
        }
    }

    fn get_r16(&self, register: &Register) -> u16 {
        match register {
            Register::Bc => self.bc.get_u16(),
            Register::De => self.de.get_u16(),
            Register::Hl => self.hl.get_u16(),
            Register::Sp => self.sp,
            _ => unreachable!(), // other registers are not 8 bits
        }
    }

    #[allow(dead_code)]
    fn get_r16_mut(&mut self, register: &Register) -> &mut u16 {
        match register {
            Register::Bc => self.bc.get_u16_mut(),
            Register::De => self.de.get_u16_mut(),
            Register::Hl => self.hl.get_u16_mut(),
            Register::Sp => &mut self.sp,
            _ => unreachable!(), // other registers are not 8 bits
        }
    }

    fn set_r16(&mut self, register: &Register, v: u16) {
        match register {
            Register::Bc => self.bc.set_u16(v),
            Register::De => self.de.set_u16(v),
            Register::Hl => self.hl.set_u16(v),
            Register::Sp => self.sp = v,
            _ => unreachable!(), // other registers are not 8 bits
        }
    }

    fn should_service_interrupt(&self, memory_bus: &mut MemoryBus) -> bool {
        // Check if interrupts are enabled
        let interrupts_enabled = memory_bus.read_u8(INTERRUPT_ENABLE_REGISTER_ADDRESS) != 0;
        if !interrupts_enabled {
            return false;
        }
        // Check if any interrupts are waiting
        memory_bus.read_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS) != 0
    }

    fn service_interrupt(&mut self, memory_bus: &mut MemoryBus) {
        // Actual hardware process (from https://gbdev.io/pandocs/Interrupts.html):
        // Clear the bit corresponding to the interrupt
        // disable interrupts
        // 2 cycles of nop
        // Push PC onto stack
        // The PC is set to the interrupt handler

        // Determine which interrupt this is. Lower bits in the interrupt flags register
        // are higher priority
        let mut interrupt_flags = memory_bus.read_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS);
        let interrupt_number = interrupt_flags.trailing_zeros() as u8;
        assert!(interrupt_number < 5);
        // Clear this bit
        Self::reset_bit(interrupt_number, &mut interrupt_flags);
        memory_bus.write_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS, interrupt_flags);
        // Disable interrupts
        self.disable_interrupts(memory_bus);

        // 2 cycles of nop does nothing
        // Calling the interrupt handler should accomplish the last two steps
        // Interrupt handler addresses are 0x40, 0x48, 0x50, 0x58, 0x60.
        self.call(memory_bus, (0x40 + 8 * interrupt_number) as u16);
    }

    pub fn dump_state(&self) {
        println!("CPU State: {:?}", self.state);
        println!("af = {} bc = {}", self.af, self.bc);
        println!("de = {} hl = {}", self.de, self.hl);
        println!("pc = {:04x} sp = {:04x}", self.pc, self.sp);
        println!("\tFlags: {}", self.dump_flags_to_string())
    }

    fn dump_flags_to_string(&self) -> String {
        format!(
            "{}{}{}{}",
            if self.get_zero_flag() { "Z" } else { "_" },
            if self.get_subtraction_flag() {
                "N"
            } else {
                "_"
            },
            if self.get_half_carry_flag() { "H" } else { "_" },
            if self.get_carry_flag() { "C" } else { "_" },
        )
    }
}
