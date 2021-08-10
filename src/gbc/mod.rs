pub mod cartridge;
pub mod cpu;
pub mod debug;
pub mod memory_bus;
pub mod mmio;
pub mod ppu;

use std::rc::Rc;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::{io, path::Path};

use cartridge::Cartridge;
use cpu::Cpu;
use debug::{AccessType, BreakReason, Breakpoint};
use memory_bus::MemoryBus;
use mmio::Mmio;
use ppu::PictureProcessingUnit;

#[derive(Debug)]
pub struct Gbc {
    running: bool,
    clock_speed: u64, // HZ
    cpu: Cpu,
    cartridge: Rc<RefCell<Cartridge>>,
    ram: Rc<RefCell<[u8; 8192]>>,
    ppu: Rc<RefCell<PictureProcessingUnit>>,
    mmio: Rc<RefCell<Mmio>>,
    high_ram: Rc<RefCell<[u8; 126]>>,
    interrupt_master_enable: Rc<RefCell<bool>>,
    cycle_count: u64,
    breakpoints: Vec<Breakpoint>,
    break_reason: Option<BreakReason>,
    interrupted: Arc<AtomicBool>,
}

impl Gbc {
    pub fn new<P: AsRef<Path>>(rom_path: P, show_instructions: bool) -> io::Result<Self> {
        let cartridge = Cartridge::new(rom_path)?;
        let interrupted = Arc::new(AtomicBool::new(false));
        let i = interrupted.clone();
        ctrlc::set_handler(move || {
            i.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");
        Ok(Gbc {
            running: false,
            clock_speed: 4194304, // TODO switch based on detected cartridge / config
            cpu: Cpu::new(show_instructions),
            cartridge: Rc::new(RefCell::new(cartridge)),
            ram: Rc::new(RefCell::new([0; 8192])),
            ppu: Rc::new(RefCell::new(PictureProcessingUnit::default())),
            mmio: Rc::new(RefCell::new(Mmio::default())),
            high_ram: Rc::new(RefCell::new([0; 126])),
            interrupt_master_enable: Rc::new(RefCell::new(false)),
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

    fn create_memory_bus(&self) -> MemoryBus {
        MemoryBus::new(
            self.cartridge.clone(),
            self.ram.clone(),
            self.ppu.clone(),
            self.mmio.clone(),
            self.high_ram.clone(),
            self.interrupt_master_enable.clone(),
        )
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

    pub fn single_step(&mut self) {
        let mut memory_bus = self.create_memory_bus();
        let cycles = self.cpu.single_step(&mut memory_bus);
        self.cycle_count += cycles;

        let interrupts = self.mmio.borrow_mut().tick(cycles);
        if interrupts.vblank { self.cpu.interrupt(&mut memory_bus, 0)}
        if interrupts.stat { self.cpu.interrupt(&mut memory_bus, 1)}
        if interrupts.timer { self.cpu.interrupt(&mut memory_bus, 2)}
        if interrupts.serial { self.cpu.interrupt(&mut memory_bus, 3)}
        if interrupts.joypad { self.cpu.interrupt(&mut memory_bus, 4)}
    }

    pub fn dump_cpu_state(&self) {
        self.cpu.dump_state();
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

        let memory_bus = self.create_memory_bus();
        for _ in 0..length {
            let insn = self
                .cpu
                .get_instruction_at_address(&memory_bus, address);
            println!("{}", insn);
            address += insn.size as u16;
        }
    }

    pub fn print_next_instruction(&self) {
        let memory_bus = self.create_memory_bus();
        let insn = self.cpu.get_next_instruction(&memory_bus);
        println!("{}", insn);
    }

    pub fn read_memory(&self, address: u16, length: u16) -> Vec<u8> {
        let memory_bus = self.create_memory_bus();
        memory_bus.read_mem(address, length)
    }

    pub fn get_cartridge(&self) -> Rc<RefCell<Cartridge>> {
        self.cartridge.clone()
    }
}
