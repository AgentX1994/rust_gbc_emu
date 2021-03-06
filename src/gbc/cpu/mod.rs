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

use crate::gbc::utils::Flag;

#[derive(Debug, Default)]
pub struct InterruptRequest {
    pub vblank: Flag,
    pub stat: Flag,
    pub timer: Flag,
    pub serial: Flag,
    pub joypad: Flag,
}

#[derive(Debug, PartialEq)]
pub enum State {
    Running,
    Halted,
    Stopped,
}

#[derive(Debug)]
pub struct Cpu {
    show_instructions: bool,
    af: RegisterStorage,
    bc: RegisterStorage,
    de: RegisterStorage,
    hl: RegisterStorage,
    pc: u16,
    sp: u16,
    state: State,
    interrupts_enabled: bool,
}

impl Default for Cpu {
    fn default() -> Self {
        Cpu {
            show_instructions: false,
            af: RegisterStorage::default(),
            bc: RegisterStorage::default(),
            de: RegisterStorage::default(),
            hl: RegisterStorage::default(),
            pc: 0x0000,
            sp: 0x0000,
            state: State::Running,
            interrupts_enabled: false,
        }
    }
}

const INTERRUPT_ENABLE_REGISTER_ADDRESS: u16 = 0xffff;
const INTERRUPT_FLAGS_REGISTER_ADDRESS: u16 = 0xff0f;

impl Cpu {
    #[must_use]
    pub fn new(show_instructions: bool) -> Self {
        Self {
            show_instructions,
            ..Self::default()
        }
    }

    pub fn reset(&mut self) {
        let show_instructions = self.show_instructions;
        *self = Self {
            show_instructions,
            ..Self::default()
        };
    }

    pub fn interrupt(&mut self, memory_bus: &mut MemoryBus, interrupt_number: u8) {
        assert!(interrupt_number < 5);
        let interrupts_enabled = memory_bus.read_u8(INTERRUPT_ENABLE_REGISTER_ADDRESS);
        let this_interrupt_enabled = (interrupts_enabled & (1 << interrupt_number)) != 0;

        // If this interrupt is enabled, then wake the CPU from Halt,
        // If the CPU is stopped, then the interrupt must be number 4 (joypad)
        if (self.state == State::Halted
            || (self.state == State::Stopped && interrupt_number == 4))
            && this_interrupt_enabled
        {
            // println!(
            //     "Un-Halted by interrupt {} ({})",
            //     interrupt_number,
            //     Self::interrupt_number_to_string(interrupt_number)
            // );
            self.state = State::Running;
        }
        let mut interrupt_flags = memory_bus.read_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS);
        Self::set_bit(interrupt_number, &mut interrupt_flags);
        memory_bus.write_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS, interrupt_flags);
    }

    pub fn request_interrupts(&mut self, memory_bus: &mut MemoryBus, requests: &InterruptRequest) {
        if requests.vblank.to_bool() {
            self.interrupt(memory_bus, 0);
        }
        if requests.stat.to_bool() {
            self.interrupt(memory_bus, 1);
        }
        if requests.timer.to_bool() {
            self.interrupt(memory_bus, 2);
        }
        if requests.serial.to_bool() {
            self.interrupt(memory_bus, 3);
        }
        if requests.joypad.to_bool() {
            self.interrupt(memory_bus, 4);
        }
    }

    #[must_use]
    pub fn get_program_counter(&self) -> u16 {
        self.pc
    }

    #[must_use]
    pub fn get_instruction_at_address(memory_bus: &mut MemoryBus, address: u16) -> Instruction {
        Instruction::new(address, memory_bus)
    }

    #[must_use]
    pub fn get_next_instruction(&self, memory_bus: &mut MemoryBus) -> Instruction {
        Self::get_instruction_at_address(memory_bus, self.pc)
    }

    pub fn single_step(&mut self, memory_bus: &mut MemoryBus) -> Option<u64> {
        if self.state != State::Running {
            return Some(1);
        }

        if self.should_service_interrupt(memory_bus) {
            self.service_interrupt(memory_bus);
            return Some(5);
        }
        let insn = self.get_next_instruction(memory_bus);
        if self.show_instructions {
            println!("{}", insn);
            self.dump_state();
        }
        self.execute_instruction(memory_bus, insn)
    }

    #[allow(clippy::cast_possible_wrap)]
    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::cast_sign_loss)]
    fn execute_instruction(&mut self, memory_bus: &mut MemoryBus, insn: Instruction) -> Option<u64> {
        self.pc += u16::from(insn.size());

        match insn.op {
            Opcode::Unknown { opcode: _ } => {
                println!("Unknown instruction! {}", insn);
                self.dump_state();
                None
            },
            Opcode::Nop => Some(4),
            Opcode::Stop => {
                self.state = State::Stopped;
                Some(4)
            }
            Opcode::Halt => {
                self.state = State::Halted;
                Some(4)
            }
            Opcode::Ld8 {
                destination,
                source,
            } => match destination {
                Operand::Register(r_dest) => match source {
                    Operand::Register(r_src) => {
                        let v = self.get_r8(&r_src);
                        self.set_r8(&r_dest, v);
                        Some(4)
                    }
                    Operand::U8(v) => {
                        self.set_r8(&r_dest, v);
                        Some(8)
                    }
                    Operand::Deref(d) => match d {
                        DerefOperand::Register(Register::Hl) => {
                            let v = memory_bus.read_u8(self.hl.get_u16());
                            self.set_r8(&r_dest, v);
                            Some(8)
                        }
                        DerefOperand::Register(r_src)
                            if r_src == Register::Bc || r_src == Register::De =>
                        {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (BC) or (DE)!");
                            }
                            let v = memory_bus.read_u8(self.get_r16(&r_src));
                            self.set_a(v);
                            Some(8)
                        }
                        DerefOperand::Register(Register::HlPlus) => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (Hl+)!");
                            }
                            let v = {
                                let hl = self.hl.get_u16_mut();
                                let temp = memory_bus.read_u8(*hl);
                                *hl = hl.wrapping_add(1);
                                temp
                            };
                            self.set_a(v);
                            Some(8)
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
                            Some(8)
                        }
                        DerefOperand::Address(addr) => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (nn)!");
                            }
                            let v = memory_bus.read_u8(addr);
                            self.set_a(v);
                            Some(16)
                        }
                        DerefOperand::Ff00Offset(offset) => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (0xff00+n)!");
                            }
                            let v = memory_bus.read_u8(0xff00_u16.wrapping_add(u16::from(offset)));
                            self.set_a(v);
                            Some(12)
                        }
                        DerefOperand::Ff00PlusC => {
                            if r_dest != Register::A {
                                panic!("Destination must be A for load from (0xff00+C)!");
                            }
                            let v = memory_bus
                                .read_u8(0xff00_u16.wrapping_add(u16::from(self.bc.get_low())));
                            self.set_a(v);
                            Some(8)
                        }
                        DerefOperand::Register(_) => unreachable!(),
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
                            Some(cycles)
                        }
                        Register::Bc | Register::De => {
                            if source != Operand::Register(Register::A) {
                                panic!("Source must be A for load to (BC) or (DE)!");
                            }
                            let v = self.get_a();
                            memory_bus.write_u8(self.get_r16(&r), v);
                            Some(8)
                        }
                        Register::HlPlus => {
                            if source != Operand::Register(Register::A) {
                                panic!("Source must be A for load to (Hl+)!");
                            }
                            let v = self.get_a();
                            let hl = self.hl.get_u16_mut();
                            memory_bus.write_u8(*hl, v);
                            *hl += 1;
                            Some(8)
                        }
                        Register::HlMinus => {
                            if source != Operand::Register(Register::A) {
                                panic!("Source must be A for load to (Hl-)!");
                            }
                            let v = self.get_a();
                            let hl = self.hl.get_u16_mut();
                            memory_bus.write_u8(*hl, v);
                            *hl -= 1;
                            Some(8)
                        }
                        _ => unreachable!(),
                    },
                    DerefOperand::Address(addr) => {
                        if source != Operand::Register(Register::A) {
                            panic!("Source must be A for load to (nn)!");
                        }
                        let v = self.get_a();
                        memory_bus.write_u8(addr, v);
                        Some(16)
                    }
                    DerefOperand::Ff00Offset(offset) => {
                        if source != Operand::Register(Register::A) {
                            panic!("Source must be A for load to (0xff00+n)!");
                        }
                        let v = self.get_a();
                        memory_bus.write_u8(0xff00_u16.wrapping_add(u16::from(offset)), v);
                        Some(8)
                    }
                    DerefOperand::Ff00PlusC => {
                        if source != Operand::Register(Register::A) {
                            panic!("Source must be A for load to (0xff00+C)!");
                        }
                        let v = self.get_a();
                        memory_bus.write_u8(0xff00_u16.wrapping_add(u16::from(self.bc.get_low())), v);
                        Some(8)
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
                            Some(12)
                        }
                        Operand::Register(Register::Hl) => {
                            if r != Register::Sp {
                                panic!("Destination must be SP for ld from HL!");
                            }
                            self.set_r16(&r, self.hl.get_u16());
                            Some(8)
                        }
                        Operand::StackOffset(d) => {
                            if r != Register::Hl {
                                panic!("Destination must be HL for ld from SP+dd!");
                            }
                            // Convert the i8 to a u16 and use overflowing add
                            let d_u8 = d.to_le_bytes()[0];
                            let d_u16 = if d < 0 {
                                u16::from_le_bytes([d_u8, 0xFF])
                            } else {
                                u16::from_le_bytes([d_u8, 0x00])
                            };
                            let (sum, _) = self.sp.overflowing_add(d_u16);
                            self.clear_zero_flag();
                            self.clear_subtraction_flag();
                            // This instruction uses carry and half carry like it was an 8 bit add
                            self.set_carry_flag_from_bool((self.sp & 0xff) + (d_u16 & 0xff) > 0xff);
                            self.set_half_carry_flag_from_bool(
                                (self.sp & 0xf) + (d_u16 & 0xf) > 0xf,
                            );
                            self.set_r16(&r, sum);
                            Some(12)
                        }
                        _ => unreachable!(),
                    }
                }
                Operand::Deref(DerefOperand::Address(addr)) => match source {
                    Operand::Register(Register::Sp) => {
                        memory_bus.write_u16(addr, self.sp);
                        Some(20)
                    }
                    _ => unreachable!(),
                },
                _ => unreachable!(),
            },
            Opcode::Jp { destination } => match destination {
                Operand::U16(address) => {
                    self.pc = address;
                    Some(16)
                }
                Operand::Register(Register::Hl) => {
                    self.pc = self.hl.get_u16();
                    Some(4)
                }
                _ => unreachable!(),
            },
            Opcode::JpCond {
                condition,
                destination,
            } => {
                if self.check_condition(&condition) {
                    self.pc = destination;
                    Some(16)
                } else {
                    Some(12)
                }
            }
            Opcode::Jr { offset } => {
                self.pc = (i32::from(self.pc) + i32::from(offset)) as u16;
                Some(12)
            }
            Opcode::JrCond { condition, offset } => {
                if self.check_condition(&condition) {
                    self.pc = (i32::from(self.pc) + i32::from(offset)) as u16;
                    Some(12)
                } else {
                    Some(8)
                }
            }
            Opcode::Call { destination } => {
                self.call(memory_bus, destination);
                Some(24)
            }
            Opcode::CallCond {
                condition,
                destination,
            } => {
                if self.check_condition(&condition) {
                    self.call(memory_bus, destination);
                    Some(24)
                } else {
                    Some(16)
                }
            }
            Opcode::Ret => {
                self.ret(memory_bus);
                Some(16)
            }
            Opcode::RetCond { condition } => {
                if self.check_condition(&condition) {
                    self.ret(memory_bus);
                    Some(20)
                } else {
                    Some(8)
                }
            }
            Opcode::Reti => {
                self.ret(memory_bus);
                self.interrupts_enabled = true;
                Some(16)
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
                        let af = self.af.get_u16_mut();
                        *af = v & 0xfff0;
                    }
                    _ => unreachable!(),
                }
                Some(12)
            }
            Opcode::Push { register } => {
                match register {
                    Register::Bc => self.push(memory_bus, self.bc.get_u16()),
                    Register::De => self.push(memory_bus, self.de.get_u16()),
                    Register::Hl => self.push(memory_bus, self.hl.get_u16()),
                    Register::Af => self.push(memory_bus, self.af.get_u16()),
                    _ => unreachable!(),
                }
                Some(16)
            }
            Opcode::Rst { vector } => {
                self.call(memory_bus, u16::from(vector));
                Some(16)
            }
            Opcode::Bit { bit, destination } => {
                // Clear subtraction flag, set half-carry flag
                self.clear_subtraction_flag();
                self.set_half_carry_flag();
                match destination {
                    Operand::Register(r) => {
                        let v = self.get_r8(&r);
                        self.clear_subtraction_flag();
                        self.set_half_carry_flag();
                        if Self::test_bit(bit, v) {
                            self.clear_zero_flag();
                        } else {
                            self.set_zero_flag();
                        }
                        Some(8)
                    }
                    Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                        let v = memory_bus.read_u8(self.hl.get_u16());
                        self.clear_subtraction_flag();
                        self.set_half_carry_flag();
                        if Self::test_bit(bit, v) {
                            self.clear_zero_flag();
                        } else {
                            self.set_zero_flag();
                        }
                        Some(12)
                    }
                    _ => unreachable!(),
                }
            }
            Opcode::Res { bit, destination } => match destination {
                Operand::Register(r) => {
                    let v = self.get_r8_mut(&r);
                    Self::reset_bit(bit, v);
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    Self::reset_bit(bit, &mut v);
                    memory_bus.write_u8(self.hl.get_u16(), v);
                    Some(16)
                }
                _ => unreachable!(),
            },
            Opcode::Set { bit, destination } => match destination {
                Operand::Register(r) => {
                    let v = self.get_r8_mut(&r);
                    Self::set_bit(bit, v);
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    Self::set_bit(bit, &mut v);
                    memory_bus.write_u8(self.hl.get_u16(), v);
                    Some(16)
                }
                _ => unreachable!(),
            },
            Opcode::Add8 { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                self.add8_with_carry(v, false, false);
                Some(cycles)
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
                            self.clear_subtraction_flag();
                            self.set_carry_flag_from_bool(carry);
                            self.set_half_carry_flag_from_bool((hl & 0xfff) + (v & 0xfff) > 0xfff);
                            self.hl.set_u16(sum);
                            Some(8)
                        } else {
                            unreachable!()
                        }
                    }
                    Register::Sp => {
                        if let Operand::I8(d) = operand {
                            // Convert the i8 to a u16 and use overflowing add
                            let d_u8 = d.to_le_bytes()[0];
                            let d_u16 = if d < 0 {
                                u16::from_le_bytes([d_u8, 0xFF])
                            } else {
                                u16::from_le_bytes([d_u8, 0x00])
                            };
                            let (sum, _) = self.sp.overflowing_add(d_u16);
                            self.clear_zero_flag();
                            self.clear_subtraction_flag();
                            // This instruction uses carry and half carry like it was an 8 bit add
                            self.set_carry_flag_from_bool((self.sp & 0xff) + (d_u16 & 0xff) > 0xff);
                            self.set_half_carry_flag_from_bool(
                                (self.sp & 0xf) + (d_u16 & 0xf) > 0xf,
                            );
                            self.sp = sum;
                            Some(16)
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
                    self.clear_subtraction_flag();
                    self.set_zero_flag_from_bool(res == 0);
                    self.set_half_carry_flag_from_bool((v & 0xf) == 0xf);
                    Some(4)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let v = memory_bus.read_u8(self.hl.get_u16());
                    let (res, _) = v.overflowing_add(1);
                    memory_bus.write_u8(self.hl.get_u16(), res);
                    self.clear_subtraction_flag();
                    self.set_zero_flag_from_bool(res == 0);
                    self.set_half_carry_flag_from_bool((v & 0xf) == 0xf);
                    Some(12)
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
                Some(8)
            }
            Opcode::Dec { operand } => match operand {
                Operand::Register(r) => {
                    let v = self.get_r8(&r);
                    let (res, _) = v.overflowing_add(0xff); // - 1 is the same as + 0xff
                    self.set_r8(&r, res);
                    self.set_zero_flag_from_bool(res == 0);
                    self.set_subtraction_flag();
                    self.set_half_carry_flag_from_bool((res & 0xf) == 0xf);
                    Some(4)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let v = memory_bus.read_u8(self.hl.get_u16());
                    let (res, _) = v.overflowing_add(0xff); // - 1 is the same as + 0xff
                    memory_bus.write_u8(self.hl.get_u16(), res);
                    self.set_zero_flag_from_bool(res == 0);
                    self.set_subtraction_flag();
                    self.set_half_carry_flag_from_bool((res & 0xf) == 0xf);
                    Some(12)
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
                Some(8)
            }
            Opcode::Adc { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                self.add8_with_carry(v, true, false);
                Some(cycles)
            }
            Opcode::Sub { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                self.add8_with_carry(v, false, true);
                // Toggle carry
                // self.set_carry_flag_from_bool(!self.get_carry_flag());
                Some(cycles)
            }
            Opcode::Sbc { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                self.add8_with_carry(v, true, true);
                // Toggle carry
                // self.set_carry_flag_from_bool(!self.get_carry_flag());
                Some(cycles)
            }
            Opcode::And { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                let val = {
                    let a = self.get_a_mut();
                    *a &= v;
                    *a
                };
                self.set_zero_flag_from_bool(val == 0);
                self.clear_subtraction_flag();
                self.set_half_carry_flag();
                self.clear_carry_flag();
                Some(cycles)
            }
            Opcode::Xor { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                let val = {
                    let a = self.get_a_mut();
                    *a ^= v;
                    *a
                };
                self.set_zero_flag_from_bool(val == 0);
                self.clear_subtraction_flag();
                self.clear_carry_flag();
                self.clear_half_carry_flag();
                Some(cycles)
            }
            Opcode::Or { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);

                let val = {
                    let a = self.get_a_mut();
                    *a |= v;
                    *a
                };
                self.set_zero_flag_from_bool(val == 0);
                self.clear_subtraction_flag();
                self.clear_carry_flag();
                self.clear_half_carry_flag();
                Some(cycles)
            }
            Opcode::Cp { operand } => {
                let (v, cycles) = self.extract_u8_arithmetic_operand(memory_bus, operand);
                // Compute A - v, but only set flags according to the result
                let a = self.get_a();
                self.set_zero_flag_from_bool(a == v);
                self.set_subtraction_flag();
                self.set_carry_flag_from_bool(a < v);
                self.set_half_carry_flag_from_bool((a % 16) < (v % 16));
                Some(cycles)
            }
            Opcode::Cpl => {
                self.set_a(!self.get_a());
                self.set_subtraction_flag();
                self.set_half_carry_flag();
                Some(4)
            }
            Opcode::Daa => {
                // from https://forums.nesdev.com/viewtopic.php?t=15944
                // If not subtraction
                let mut a = self.get_a();
                if self.get_subtraction_flag() {
                    if self.get_carry_flag() {
                        a = a.wrapping_sub(0x60);
                        self.set_carry_flag();
                    }
                    if self.get_half_carry_flag() {
                        a = a.wrapping_sub(0x6);
                    }
                } else {
                    if self.get_carry_flag() || a > 0x99 {
                        a = a.wrapping_add(0x60);
                        self.set_carry_flag();
                    }
                    if self.get_half_carry_flag() || (a & 0xf) > 0x9 {
                        a = a.wrapping_add(0x6);
                    }
                }

                self.set_zero_flag_from_bool(a == 0);
                self.clear_half_carry_flag();
                self.set_a(a);
                Some(4)
            }
            Opcode::Rlca => {
                let mut a = self.get_a();
                let high_bit = a >> 7;
                let carry = high_bit == 1;
                a = (a << 1) | high_bit;
                self.set_a(a);

                // only the carry flag should be set after this;
                self.clear_flags();
                self.set_carry_flag_from_bool(carry);
                Some(4)
            }
            Opcode::Rla => {
                let mut a = self.get_a();
                let old_carry = self.get_carry_flag() as u8;
                let high_bit = a >> 7;
                let carry = high_bit == 1;
                a = (a << 1) | old_carry;
                self.set_a(a);

                // only the carry flag should be set after this;
                self.clear_flags();
                self.set_carry_flag_from_bool(carry);
                Some(4)
            }
            Opcode::Rrca => {
                let mut a = self.get_a();
                let low_bit = a & 1;
                let carry = low_bit == 1;
                a = (a >> 1) | (low_bit << 7);
                self.set_a(a);

                // only the carry flag should be set after this;
                self.clear_flags();
                self.set_carry_flag_from_bool(carry);
                Some(4)
            }
            Opcode::Rra => {
                let mut a = self.get_a();
                let old_carry = self.get_carry_flag() as u8;
                let low_bit = a & 1;
                let carry = low_bit == 1;
                a = (a >> 1) | (old_carry << 7);
                self.set_a(a);

                // only the carry flag should be set after this;
                self.clear_flags();
                self.set_carry_flag_from_bool(carry);
                Some(4)
            }
            Opcode::Rlc { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = (v << 1) | high_bit;
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = (v << 1) | high_bit;
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(16)
                }
                _ => unreachable!(),
            },
            Opcode::Rl { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let old_carry = self.get_carry_flag() as u8;
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = (v << 1) | old_carry;
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let old_carry = self.get_carry_flag() as u8;
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = (v << 1) | old_carry;
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(16)
                }
                _ => unreachable!(),
            },
            Opcode::Rrc { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = (v >> 1) | (low_bit << 7);
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = (v >> 1) | (low_bit << 7);
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(16)
                }
                _ => unreachable!(),
            },
            Opcode::Rr { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let old_carry = self.get_carry_flag() as u8;
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = (v >> 1) | (old_carry << 7);
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let old_carry = self.get_carry_flag() as u8;
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v = (v >> 1) | (old_carry << 7);
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(16)
                }
                _ => unreachable!(),
            },
            Opcode::Sla { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = v.wrapping_mul(2);
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let high_bit = v >> 7;
                    let carry = high_bit == 1;
                    v = v.wrapping_mul(2);
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(16)
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
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    v = (v >> 4) | (v << 4);
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the zero flag should be set after this;
                    self.clear_flags();
                    self.set_zero_flag_from_bool(v == 0);
                    Some(16)
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
                    Some(8)
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
                    Some(16)
                }
                _ => unreachable!(),
            },
            Opcode::Srl { operand } => match operand {
                Operand::Register(r) => {
                    let mut v = self.get_r8(&r);
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v >>= 1;
                    self.set_r8(&r, v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(8)
                }
                Operand::Deref(DerefOperand::Register(Register::Hl)) => {
                    let mut v = memory_bus.read_u8(self.hl.get_u16());
                    let low_bit = v & 1;
                    let carry = low_bit == 1;
                    v >>= 1;
                    memory_bus.write_u8(self.hl.get_u16(), v);

                    // only the carry and zero flags should be set after this;
                    self.clear_flags();
                    self.set_carry_flag_from_bool(carry);
                    self.set_zero_flag_from_bool(v == 0);
                    Some(16)
                }
                _ => unreachable!(),
            },
            Opcode::Scf => {
                // Sets carry to 1, clears half carry and subtraction
                self.set_carry_flag();
                self.clear_half_carry_flag();
                self.clear_subtraction_flag();
                Some(4)
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
                Some(4)
            }
            Opcode::Di => {
                self.interrupts_enabled = false;
                Some(4)
            }
            Opcode::Ei => {
                self.interrupts_enabled = true;
                Some(4)
            }
        }
    }

    fn check_condition(&self, condition: &ConditionType) -> bool {
        match condition {
            instruction::ConditionType::NonZero => !self.get_zero_flag(),
            instruction::ConditionType::Zero => self.get_zero_flag(),
            instruction::ConditionType::NotCarry => !self.get_carry_flag(),
            instruction::ConditionType::Carry => self.get_carry_flag(),
        }
    }

    fn push(&mut self, memory_bus: &mut MemoryBus, v: u16) {
        self.sp = self.sp.wrapping_sub(2);
        memory_bus.write_mem(self.sp, &v.to_le_bytes()[..]);
    }

    fn pop(&mut self, memory_bus: &mut MemoryBus) -> u16 {
        let v = memory_bus.read_mem(self.sp, 2);
        self.sp = self.sp.wrapping_add(2);
        assert!(v.len() == 2);
        (u16::from(v[1]) << 8) | u16::from(v[0])
    }

    fn call(&mut self, memory_bus: &mut MemoryBus, address: u16) {
        self.push(memory_bus, self.pc);
        self.pc = address;
    }

    fn ret(&mut self, memory_bus: &mut MemoryBus) {
        self.pc = self.pop(memory_bus);
    }

    fn test_bit(bit: u8, v: u8) -> bool {
        assert!(bit < 8);
        (v & (1 << bit)) != 0
    }

    fn set_bit(bit: u8, v: &mut u8) {
        assert!(bit < 8);
        *v |= 1 << bit;
    }

    fn reset_bit(bit: u8, v: &mut u8) {
        assert!(bit < 8);
        *v &= !(1 << bit);
    }

    fn get_flags(&self) -> u8 {
        self.af.get_low()
    }

    fn set_flags(&mut self, flags: u8) {
        self.af.set_low(flags);
    }

    fn add8_with_carry(&mut self, b: u8, use_carry: bool, subtraction: bool) {
        let a = self.af.get_high();
        let c = if use_carry && self.get_carry_flag() {
            1_u8
        } else {
            0_u8
        };
        let operand = if subtraction { (!b).wrapping_add(1) } else { b };
        let c_operand = if subtraction { (!c).wrapping_add(1) } else { c };
        let (sum, carry1) = a.overflowing_add(operand);
        let (sum, carry2) = sum.overflowing_add(c_operand);
        let zero = sum == 0;
        let half_carry = if subtraction {
            if use_carry && c == 1 && (b & 0xf) == 0xf {
                true
            } else {
                (a & 0xf) < ((b & 0xf) + (c & 0xf))
            }
        } else {
            ((a & 0xf) + (b & 0xf) + c) > 0xf
        };
        let carry = if subtraction {
            if use_carry && c == 1 && b == 0xff {
                true
            } else {
                a < (b + c)
            }
        } else {
            carry1 || carry2
        };
        self.af.set_high(sum as u8);
        self.set_zero_flag_from_bool(zero);
        self.set_subtraction_flag_from_bool(subtraction);
        self.set_carry_flag_from_bool(carry);
        self.set_half_carry_flag_from_bool(half_carry);
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
            self.set_zero_flag();
        } else {
            self.clear_zero_flag();
        }
    }

    #[allow(dead_code)]
    fn set_subtraction_flag_from_bool(&mut self, subtraction: bool) {
        if subtraction {
            self.set_subtraction_flag();
        } else {
            self.clear_subtraction_flag();
        }
    }

    #[allow(dead_code)]
    fn set_half_carry_flag_from_bool(&mut self, half_carry: bool) {
        if half_carry {
            self.set_half_carry_flag();
        } else {
            self.clear_half_carry_flag();
        }
    }

    fn set_carry_flag_from_bool(&mut self, carry: bool) {
        if carry {
            self.set_carry_flag();
        } else {
            self.clear_carry_flag();
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
        if !self.interrupts_enabled {
            return false;
        }
        // Check if interrupts are enabled
        let enabled_interrupts_bitmask = memory_bus.read_u8(INTERRUPT_ENABLE_REGISTER_ADDRESS);
        if enabled_interrupts_bitmask == 0 {
            return false;
        }
        // Check if any interrupts are waiting
        (memory_bus.read_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS) & enabled_interrupts_bitmask) != 0
    }

    #[allow(dead_code)]
    fn interrupt_number_to_string(number: u8) -> &'static str {
        match number {
            0 => "vblank",
            1 => "lcd stat",
            2 => "timer",
            3 => "serial",
            4 => "joypad",
            _ => "unknown",
        }
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
        let mut interrupt_flags = memory_bus.read_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS)
            & memory_bus.read_u8(INTERRUPT_ENABLE_REGISTER_ADDRESS);
        #[allow(clippy::cast_possible_truncation)]
        let interrupt_number = interrupt_flags.trailing_zeros() as u8;
        assert!(interrupt_number < 5);
        // println!(
        //     "Servicing interrupt #{} ({})",
        //     interrupt_number,
        //     Self::interrupt_number_to_string(interrupt_number)
        // );
        // Clear this bit
        Self::reset_bit(interrupt_number, &mut interrupt_flags);
        memory_bus.write_u8(INTERRUPT_FLAGS_REGISTER_ADDRESS, interrupt_flags);
        // Disable interrupts
        self.interrupts_enabled = false;

        // 2 cycles of nop does nothing
        // Calling the interrupt handler should accomplish the last two steps
        // Interrupt handler addresses are 0x40, 0x48, 0x50, 0x58, 0x60.
        self.call(memory_bus, u16::from(0x40 + 8 * interrupt_number));
    }

    pub fn dump_state(&self) {
        println!("\tCPU State: {:?}", self.state);
        println!("\taf = {} bc = {}", self.af, self.bc);
        println!("\tde = {} hl = {}", self.de, self.hl);
        println!("\tpc = {:04x} sp = {:04x}", self.pc, self.sp);
        println!("\t\tFlags: {}", self.dump_flags_to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_default_memory_bus() -> MemoryBus {
        use crate::gbc::cartridge::Cartridge;
        let cartridge = Cartridge::default();

        MemoryBus::new(cartridge)
    }

    #[test]
    fn test_default_cpu() {
        let cpu = Cpu::default();
        assert_eq!(cpu.af.get_high(), 1);
        assert_eq!(cpu.sp, 0xfffe);
    }

    #[test]
    #[should_panic]
    fn test_unknown_instruction() {
        let mut cpu = Cpu::default();
        let mut memory_bus = create_default_memory_bus();
        cpu.execute_instruction(
            &mut memory_bus,
            Instruction {
                address: 0,
                op: Opcode::Unknown { opcode: 0 },
            },
        );
    }

    #[test]
    fn test_add_8() {
        let mut cpu = Cpu::default();
        cpu.af.set_high(0);
        cpu.add8_with_carry(0, false, false);
        assert_eq!(cpu.af.get_high(), 0);
        assert!(cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(!cpu.get_half_carry_flag());
        assert!(!cpu.get_carry_flag());

        cpu.af.set_high(0xff);
        cpu.add8_with_carry(1, false, false);
        assert_eq!(cpu.af.get_high(), 0);
        assert!(cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(cpu.get_half_carry_flag());
        assert!(cpu.get_carry_flag());

        cpu.add8_with_carry(0xff, true, false);
        assert_eq!(cpu.af.get_high(), 0);
        assert!(cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(cpu.get_half_carry_flag());
        assert!(cpu.get_carry_flag());

        cpu.af.set_high(0xf);
        cpu.add8_with_carry(1, false, false);
        assert_eq!(cpu.af.get_high(), 0x10);
        assert!(!cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(cpu.get_half_carry_flag());
        assert!(!cpu.get_carry_flag());

        cpu.af.set_high(0xf);
        cpu.add8_with_carry(0xf, false, true);
        assert_eq!(cpu.af.get_high(), 0);
        assert!(cpu.get_zero_flag());
        assert!(cpu.get_subtraction_flag());
        assert!(!cpu.get_half_carry_flag());
        assert!(!cpu.get_carry_flag());

        cpu.set_carry_flag();
        cpu.af.set_high(0xff);
        cpu.add8_with_carry(0xff, true, false);
        assert_eq!(cpu.af.get_high(), 0xff);
        assert!(!cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(cpu.get_half_carry_flag());
        assert!(cpu.get_carry_flag());

        cpu.set_carry_flag();
        cpu.af.set_high(0xf0);
        cpu.add8_with_carry(0xf, true, false);
        assert_eq!(cpu.af.get_high(), 0);
        assert!(cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(cpu.get_half_carry_flag());
        assert!(cpu.get_carry_flag());

        cpu.af.set_high(2);
        cpu.add8_with_carry(1, false, true);
        assert_eq!(cpu.af.get_high(), 1);
        assert!(!cpu.get_zero_flag());
        assert!(cpu.get_subtraction_flag());
        assert!(!cpu.get_half_carry_flag());
        assert!(!cpu.get_carry_flag());
    }

    #[test]
    fn test_add_sp_i8() {
        let mut cpu = Cpu::default();

        let mut memory_bus = create_default_memory_bus();

        let insn = Instruction {
            address: 0,
            op: Opcode::Add16 {
                register: Register::Sp,
                operand: Operand::I8(-1),
            },
        };
        cpu.sp = 0x8000;
        cpu.execute_instruction(&mut memory_bus, insn);
        assert_eq!(cpu.sp, 0x7fff);
        assert!(!cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(!cpu.get_half_carry_flag());
        assert!(!cpu.get_carry_flag());

        let insn = Instruction {
            address: 0,
            op: Opcode::Add16 {
                register: Register::Sp,
                operand: Operand::I8(-2),
            },
        };
        cpu.sp = 0x8000;
        cpu.execute_instruction(&mut memory_bus, insn);
        assert_eq!(cpu.sp, 0x7ffe);
        assert!(!cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(!cpu.get_half_carry_flag());
        assert!(!cpu.get_carry_flag());

        let insn = Instruction {
            address: 0,
            op: Opcode::Add16 {
                register: Register::Sp,
                operand: Operand::I8(2),
            },
        };
        cpu.sp = 0x8000;
        cpu.execute_instruction(&mut memory_bus, insn);
        assert_eq!(cpu.sp, 0x8002);
        assert!(!cpu.get_zero_flag());
        assert!(!cpu.get_subtraction_flag());
        assert!(!cpu.get_half_carry_flag());
        assert!(!cpu.get_carry_flag());
    }

    #[test]
    fn test_push_pop() {
        let mut cpu = Cpu::default();
        let mut memory_bus = create_default_memory_bus();
        // sp defaults to 0xfffe
        let v = 0x1234u16;
        cpu.push(&mut memory_bus, v);
        assert_eq!(cpu.sp, 0xfffc);
        assert_eq!(memory_bus.read_u16(cpu.sp), v);
        let v_popped = cpu.pop(&mut memory_bus);
        assert_eq!(v, v_popped);
        assert_eq!(cpu.sp, 0xfffe);
    }

    #[test]
    fn test_pop_af() {
        let mut cpu = Cpu::default();
        let mut memory_bus = create_default_memory_bus();
        let v = 0xffff;
        cpu.push(&mut memory_bus, v);
        let insn = Instruction {
            address: 0,
            op: Opcode::Pop {
                register: Register::Af,
            },
        };
        assert_eq!(cpu.af.get_u16(), 0x01b0);
        cpu.execute_instruction(&mut memory_bus, insn);
        assert_eq!(cpu.af.get_u16(), 0xfff0);
    }

    #[test]
    fn test_left_rotates() {
        let mut failed = false;
        let mut cpu = Cpu::default();
        let mut memory_bus = create_default_memory_bus();

        // RLCA
        for byte in 0..=255u8 {
            cpu.set_a(byte);
            cpu.execute_instruction(
                &mut memory_bus,
                Instruction {
                    address: 0,
                    op: Opcode::Rlca,
                },
            );
            let a = cpu.get_a();
            if a != byte.rotate_left(1) {
                println!(
                    "ERROR: RLCA of {:#02x} should be {:#02x}, got {:02x}",
                    byte,
                    byte.rotate_left(1),
                    a
                );
                failed = true;
            }
            let flags = if byte & 0x80 != 0 { CARRY_BIT_MASK } else { 0 };
            if cpu.get_flags() != flags {
                println!("Flags should be {}, got {}", flags, cpu.get_flags());
                failed = true;
            }
        }

        // TODO the rest of the rotates

        if failed {
            panic!("FAILED!");
        }
    }

    #[test]
    fn test_sub() {
        let mut cpu = Cpu::default();
        let mut memory_bus = create_default_memory_bus();
        const VALUES: [u8; 9] = [0x00, 0x01, 0x0F, 0x10, 0x1F, 0x7F, 0x80, 0xF0, 0xFF];
        let mut failed = false;
        for a in VALUES {
            for imm in VALUES {
                let insn = Instruction {
                    address: 0,
                    op: Opcode::Sub {
                        operand: Operand::U8(imm),
                    },
                };

                cpu.set_a(a);
                cpu.execute_instruction(&mut memory_bus, insn);
                let real_res = a.wrapping_sub(imm);
                let res = cpu.get_a();
                if res != real_res {
                    println!(
                        "Error: {:#02x} - {:#02x} results in {:#02x} instead of {:#02x}",
                        a, imm, res, real_res
                    );
                    failed = true;
                }

                let mut flags = SUBTRACTION_BIT_MASK;
                if real_res == 0 {
                    flags |= ZERO_BIT_MASK;
                }
                if (a & 0xf) < (imm & 0xf) {
                    flags |= HALF_CARRY_BIT_MASK;
                }
                if imm > a {
                    flags |= CARRY_BIT_MASK;
                }
                if flags != cpu.get_flags() {
                    let cpu_flags_string = cpu.dump_flags_to_string();
                    let cpu_flags = cpu.get_flags();
                    cpu.set_flags(flags);
                    let real_flags_string = cpu.dump_flags_to_string();
                    cpu.set_flags(cpu_flags);
                    println!(
                        "Error: {} - {}: Flags should be {}, got {}",
                        a, imm, real_flags_string, cpu_flags_string
                    );
                    failed = true;
                }
            }
        }

        if failed {
            panic!("FAILED");
        }
    }

    #[test]
    fn test_sbc() {
        let mut cpu = Cpu::default();
        let mut memory_bus = create_default_memory_bus();
        const VALUES: [u8; 9] = [0x00, 0x01, 0x0F, 0x10, 0x1F, 0x7F, 0x80, 0xF0, 0xFF];
        let mut failed = false;
        for a in VALUES {
            for imm in VALUES {
                for carry in [false, true] {
                    let insn = Instruction {
                        address: 0,
                        op: Opcode::Sbc {
                            operand: Operand::U8(imm),
                        },
                    };

                    cpu.set_a(a);
                    cpu.set_carry_flag_from_bool(carry);
                    cpu.execute_instruction(&mut memory_bus, insn);
                    let real_res = a.wrapping_sub(imm).wrapping_sub(carry as u8);
                    let res = cpu.get_a();
                    if res != real_res {
                        println!(
                            "Error: {:#02x} - {:#02x} - {} results in {:#02x} instead of {:#02x}",
                            a, imm, carry as u8, res, real_res,
                        );
                        failed = true;
                    }

                    let mut flags = SUBTRACTION_BIT_MASK;
                    if real_res == 0 {
                        flags |= ZERO_BIT_MASK;
                    }
                    if (a & 0xf) < (imm & 0xf) || (a & 0xf) < ((imm & 0xf) + (carry as u8 & 0xf)) {
                        flags |= HALF_CARRY_BIT_MASK;
                    }
                    if imm > a || (imm as u16 + carry as u16) > a as u16 {
                        flags |= CARRY_BIT_MASK;
                    }
                    if flags != cpu.get_flags() {
                        let cpu_flags_string = cpu.dump_flags_to_string();
                        let cpu_flags = cpu.get_flags();
                        cpu.set_flags(flags);
                        let real_flags_string = cpu.dump_flags_to_string();
                        cpu.set_flags(cpu_flags);
                        println!(
                            "Error: {} - {} - {}: Flags should be {}, got {}",
                            a, imm, carry as u8, real_flags_string, cpu_flags_string
                        );
                        failed = true;
                    }
                }
            }
        }

        if failed {
            panic!("FAILED");
        }
    }
}
