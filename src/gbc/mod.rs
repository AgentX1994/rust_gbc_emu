pub mod cartridge;
pub mod cpu;
pub mod debug;
pub mod memory_bus;
pub mod mmio;
pub mod ppu;

use std::cmp::min;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{io, path::Path};

use cartridge::Cartridge;
use cpu::instruction::{Instruction, Opcode, Operand};
use cpu::Cpu;
use debug::{AccessType, BreakReason, Breakpoint};
use memory_bus::MemoryBus;

#[derive(Debug)]
pub struct Gbc {
    running: bool,
    memory_bus: MemoryBus,
    cpu: Cpu,
    cycle_count: u64,
    breakpoints: Vec<Breakpoint>,
    break_reason: Option<BreakReason>,
    interrupted: Arc<AtomicBool>
}

impl Gbc {
    pub fn new<P: AsRef<Path>>(rom_path: P) -> io::Result<Self> {
        let cartridge = Cartridge::new(rom_path)?;
        let interrupted = Arc::new(AtomicBool::new(false));
        let i = interrupted.clone();
        ctrlc::set_handler(move || {
            i.store(true, Ordering::SeqCst);
        }).expect("Error setting Ctrl-C handler");
        Ok(Gbc {
            running: false,
            memory_bus: MemoryBus::new(cartridge),
            cpu: Cpu::default(),
            cycle_count: 0,
            breakpoints: Vec::new(),
            break_reason: None,
            interrupted,
        })
    }

    pub fn add_breakpoint(
        &mut self,
        address: u16,
        access_type: AccessType,
        length: u16,
        reason: BreakReason,
    ) {
        let bp = Breakpoint::new(address, access_type, length, reason);
        self.breakpoints.push(bp);
    }

    pub fn list_breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints[..]
    }

    pub fn remove_breakpoint(&mut self, index: usize) {
        if index < self.breakpoints.len() {
            self.breakpoints.remove(index);
        } else {
            println!("Unknown index {}", index);
        }
    }

    fn check_execute_breakpoints(&mut self) {
        let pc = self.cpu.get_program_counter();
        // println!("\tCurrent pc: {}", pc);
        for bp in self.breakpoints.iter() {
            // println!("\tChecking breakpoint {} {} {} {}", bp.address, bp.access_type, bp.length, bp.reason);
            if !bp.access_type.on_execute() {
                // println!("\tNot an execute bp");
                continue;
            }

            if !bp.matches_address(pc) {
                // println!("\tNot an matching address");
                continue;
            }

            // println!("\thit!");
            self.running = false;
            self.break_reason = Some(bp.reason);
            return;
        }
    }

    pub fn run(&mut self) {
        // clear any previous interrupts
        self.interrupted.store(false, Ordering::SeqCst);
        self.running = true;
        while self.running {
            self.single_step();
            self.check_execute_breakpoints();
            if self.interrupted.load(Ordering::SeqCst) {
                self.running = false;
                println!();
            }
        }
    }

    pub fn dump_instructions(&self) {
        let entrypoint = &self.memory_bus.cartridge.rom[0x0100..0x0104];

        // Print the entrypoint bytes
        for b in entrypoint {
            print!("{:x}", b);
            if b != entrypoint.last().expect("entrypoint has no bytes!") {
                print!(" ");
            }
        }

        println!();

        // The entrypoint region usually contains
        // a nop, followed by a jp 0x0150
        let mut start = 0;
        let mut actual_entrypoint = None;
        while start < entrypoint.len() {
            let end = min(start + 3, entrypoint.len());
            let bytes = &entrypoint[start..end];
            let insn = Instruction::new(0x100 + start as u16, bytes);
            println!("{}", insn);
            start += insn.size as usize;

            if let Opcode::Jp { destination } = insn.op {
                if let Operand::U16(address) = destination {
                    actual_entrypoint = Some(address);
                }
            }
        }

        assert!(actual_entrypoint.is_some());

        let actual_entrypoint = actual_entrypoint.unwrap();
        println!("After jump to {:x}:", actual_entrypoint);
        let mut number = 0;
        let mut address = actual_entrypoint as usize;
        while number < 10 {
            let end = min(address + 3, self.memory_bus.cartridge.rom.len());
            let bytes = &self.memory_bus.cartridge.rom[address..end];
            let insn = Instruction::new(address as u16, bytes);
            println!("{}", insn);
            address += insn.size as usize;
            number += 1;
        }
    }

    pub fn single_step(&mut self) {
        let cycles = self.cpu.single_step(&mut self.memory_bus);
        self.cycle_count += cycles;
    }

    pub fn dump_state(&self) {
        println!(
            "GBC State: {}",
            if self.running { "Running" } else { "Stopped" }
        );
        self.cpu.dump_state();
        println!("\tCycles run: {}", self.cycle_count);
        println!("\tBreakpoints: ");
        if self.breakpoints.len() == 0 {
            println!("\t\tNone");
        } else {
            for bp in self.breakpoints.iter() {
                println!("\t\t{:04x} {} {}", bp.address, bp.access_type, bp.reason);
            }
        }
        print!("\tBreak Reason: ");
        match self.break_reason {
            Some(ref reason) => println!("{}", reason),
            None => println!("None"),
        }
    }

    pub fn print_instructions(&self, address: Option<u16>, length: u16) {
        let mut address = if let Some(address) = address {
            address
        } else {
            self.cpu.get_program_counter()
        };

        for _ in 0..length {
            let insn = self.cpu.get_instruction_at_address(&self.memory_bus, address);
            println!("{}", insn);
            address += insn.size as u16;
        }
    }

    pub fn print_next_instruction(&self) {
        let insn = self.cpu.get_next_instruction(&self.memory_bus);
        println!("{}", insn);
    }

    pub fn read_memory(&self, address: u16, length: u16) -> Vec<u8> {
        self.memory_bus.read_mem(address, length)
    }

    pub fn get_cartridge(&self) -> &Cartridge {
        self.memory_bus.get_cartridge()
    }
}
