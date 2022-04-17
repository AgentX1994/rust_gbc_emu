pub mod cartridge;
pub mod cpu;
pub mod debug;
pub mod memory_bus;
pub mod mmio;
pub mod ppu;
pub mod utils;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{io, path::Path};

use cartridge::Cartridge;
use cpu::Cpu;
use debug::{AccessType, BreakReason, Breakpoint};
use memory_bus::MemoryBus;
use mmio::lcd::Color;

use self::cpu::InterruptRequest;
use self::ppu::{Tile, TileAddressingMethod};

#[derive(Debug)]
pub struct Gbc {
    running: Arc<AtomicBool>,
    turbo: bool,
    framebuffer: Arc<Mutex<[[Color; 160]; 144]>>,
    clock_speed: u64, // HZ
    cpu: Cpu,
    cycle_count: u64,
    breakpoints: Vec<Breakpoint>,
    break_reason: Option<Breakpoint>,
    memory_bus: MemoryBus,
}

impl Gbc {
    pub fn new<P: AsRef<Path>>(
        rom_path: P,
        framebuffer: Arc<Mutex<[[Color; 160]; 144]>>,
        running: Arc<AtomicBool>,
        turbo: bool,
        show_instructions: bool,
    ) -> io::Result<Self> {
        let cartridge = Cartridge::new(rom_path)?;
        Ok(Gbc {
            running,
            turbo,
            framebuffer,
            clock_speed: 4_194_304, // TODO switch based on detected cartridge / config
            cpu: Cpu::new(show_instructions),
            cycle_count: 0,
            breakpoints: Vec::new(),
            break_reason: None,
            memory_bus: MemoryBus::new(cartridge),
        })
    }

    #[must_use]
    pub fn get_clock_speed(&self) -> u64 {
        self.clock_speed
    }

    pub fn add_breakpoint(
        &mut self,
        address: u16,
        access_type: AccessType,
        length: u16,
        reason: BreakReason,
    ) {
        let bp = Breakpoint::new(address, access_type, length, reason);
        if access_type.on_execute() {
            self.breakpoints.push(bp);
        }
        if access_type.on_read() || access_type.on_write() {
            self.memory_bus.add_breakpoint(bp);
        }
    }

    #[must_use]
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

    #[must_use]
    pub fn get_last_breakpoint(&self) -> Option<Breakpoint> {
        self.break_reason
    }

    fn check_execute_breakpoints(&mut self) {
        let pc = self.cpu.get_program_counter();
        for bp in &self.breakpoints {
            if !bp.access_type.on_execute() {
                continue;
            }

            if !bp.matches_address(pc) {
                continue;
            }

            self.break_reason = Some(*bp);
            return;
        }
    }

    fn check_breakpoints(&mut self) {
        self.check_execute_breakpoints();
        if self.break_reason.is_none() {
            self.break_reason = self.memory_bus.get_break_reason();
        }
        if self.break_reason.is_some() {
            self.running.store(false, Ordering::Relaxed);
        }
    }

    pub fn run(&mut self) -> (u64, bool) {
        self.break_reason = None;
        self.running.store(true, Ordering::Relaxed);
        let mut cycles_in_this_run = 0;
        let mut encountered_problem = false;
        let mut start = Instant::now();
        while self.running.load(Ordering::Relaxed) {
            let cycles = match self.single_step() {
                Some(cycles) => cycles,
                None => {
                    encountered_problem = true;
                    self.running.store(false, Ordering::Relaxed);
                    break;
                }
            };
            self.cycle_count += cycles;
            cycles_in_this_run += cycles;
            self.check_breakpoints();
            let desired_iteration_time =
                Duration::from_nanos(cycles * (1_000_000_000_u64 / self.clock_speed));
            let next_cycle_time = start + desired_iteration_time;
            if !self.turbo {
                while Instant::now() < next_cycle_time {}
            }
            start = Instant::now();
        }
        (cycles_in_this_run, encountered_problem)
    }

    pub fn single_step(&mut self) -> Option<u64> {
        match self.cpu.single_step(&mut self.memory_bus) {
            Some(cycles) => {
                let interrupts = self.tick_hardware(cycles);
                self.cpu
                    .request_interrupts(&mut self.memory_bus, &interrupts);
                Some(cycles)
            }
            None => None,
        }
    }

    pub fn tick_hardware(&mut self, cycles: u64) -> InterruptRequest {
        let mut interrupts = InterruptRequest {
            serial: self.memory_bus.serial.tick(cycles).into(),
            ..InterruptRequest::default()
        };

        let lcd = &mut self.memory_bus.lcd;
        let vblank_and_stat = self.memory_bus.ppu.tick(cycles, lcd);
        interrupts.vblank = vblank_and_stat.0.into();
        interrupts.stat = vblank_and_stat.1.into();
        interrupts.timer = self.memory_bus.timer_control.tick(cycles).into();

        self.memory_bus.run_dma(cycles);

        // Update framebuffer on vblank
        if interrupts.vblank.to_bool() {
            let mut f = self.framebuffer.lock().unwrap();
            *f = *self.memory_bus.ppu.get_current_framebuffer();
        }

        interrupts
    }

    #[must_use]
    pub fn get_current_framebuffer(&self) -> [[Color; 160]; 144] {
        *self.memory_bus.ppu.get_current_framebuffer()
    }

    pub fn dump_cpu_state(&self) {
        self.cpu.dump_state();
    }

    pub fn dump_state(&self) {
        println!(
            "GBC State: {}",
            if self.running.load(Ordering::Relaxed) {
                "Running"
            } else {
                "Stopped"
            }
        );
        self.cpu.dump_state();
        println!("\tCycles run: {}", self.cycle_count);
        println!("\tBreakpoints: ");
        if self.breakpoints.is_empty() {
            println!("\t\tNone");
        } else {
            for bp in &self.breakpoints {
                println!("\t\t{:04x} {} {}", bp.address, bp.access_type, bp.reason);
            }
        }
        print!("\tBreak Reason: ");
        match self.break_reason {
            Some(ref reason) => println!("{}", reason),
            None => println!("None"),
        }
        println!("{:#?}", self.memory_bus);
    }

    pub fn print_instructions(&mut self, address: Option<u16>, length: u16) {
        let mut address = address.map_or_else(|| self.cpu.get_program_counter(), |address| address);

        for _ in 0..length {
            let insn = Cpu::get_instruction_at_address(&mut self.memory_bus, address);
            println!("{}", insn);
            address += u16::from(insn.size());
        }
    }

    pub fn print_next_instruction(&mut self) {
        let insn = self.cpu.get_next_instruction(&mut self.memory_bus);
        println!("{}", insn);
    }

    #[must_use]
    pub fn read_memory(&mut self, address: u16, length: u16) -> Vec<u8> {
        self.memory_bus.read_mem(address, length)
    }

    #[must_use]
    pub fn get_cartridge(&self) -> &Cartridge {
        &self.memory_bus.cartridge
    }

    #[must_use]
    pub fn get_tile_map(&self, map_number: u8) -> Option<&[u8]> {
        match map_number {
            0 => Some(&self.memory_bus.ppu.video_ram.background_map_0[..]),
            1 => Some(&self.memory_bus.ppu.video_ram.background_map_1[..]),
            _ => None,
        }
    }

    #[must_use]
    pub fn get_tile(&self, tile_address: TileAddressingMethod) -> Tile {
        self.memory_bus.ppu.video_ram.read_tile(tile_address)
    }

    pub fn reset(&mut self) {
        self.cycle_count = 0;
        self.cpu.reset();
        self.memory_bus.reset();
    }
}
