pub mod cartridge;
pub mod cpu;
pub mod debug;
pub mod memory_bus;
pub mod mmio;
pub mod ppu;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::{io, path::Path};

use cartridge::Cartridge;
use cpu::Cpu;
use debug::{AccessType, BreakReason, Breakpoint};
use memory_bus::MemoryBus;
use mmio::{
    apu::Sound,
    joypad::Joypad,
    lcd::{Color, Lcd},
    serial::SerialComms,
    timer::Timer,
};
use ppu::PictureProcessingUnit;

use self::cpu::InterruptRequest;

#[derive(Debug)]
pub struct Gbc {
    running: Arc<AtomicBool>,
    framebuffer: Arc<Mutex<[[Color; 160]; 144]>>,
    clock_speed: u64, // HZ
    cpu: Cpu,
    cycle_count: u64,
    breakpoints: Vec<Breakpoint>,
    break_reason: Option<BreakReason>,
    memory_bus: MemoryBus,
}

impl Gbc {
    pub fn new<P: AsRef<Path>>(
        rom_path: P,
        framebuffer: Arc<Mutex<[[Color; 160]; 144]>>,
        running: Arc<AtomicBool>,
        show_instructions: bool,
    ) -> io::Result<Self> {
        let cartridge = Cartridge::new(rom_path)?;
        let cartridge = cartridge;
        let ram = [0; 8192];
        let ppu = PictureProcessingUnit::default();
        let joypad = Joypad::default();
        let serial = SerialComms::default();
        let timer_control = Timer::default();
        let sound = Sound::default();
        let lcd = Lcd::default();
        let vram_select = 0;
        let disable_boot_rom = false;
        let vram_dma = [0; 4];
        let color_palettes = [0; 2];
        let wram_bank_select = 0;
        let interrupt_flags = 0;
        let high_ram = [0; 127];
        let interrupt_enable = 0;
        Ok(Gbc {
            running,
            framebuffer,
            clock_speed: 4194304, // TODO switch based on detected cartridge / config
            cpu: Cpu::new(show_instructions),
            cycle_count: 0,
            breakpoints: Vec::new(),
            break_reason: None,
            memory_bus: MemoryBus::new(
                cartridge,
                ram,
                ppu,
                joypad,
                serial,
                timer_control,
                sound,
                lcd,
                vram_select,
                disable_boot_rom,
                vram_dma,
                color_palettes,
                wram_bank_select,
                interrupt_flags,
                high_ram,
                interrupt_enable,
            ),
        })
    }

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
        for bp in self.breakpoints.iter() {
            if !bp.access_type.on_execute() {
                continue;
            }

            if !bp.matches_address(pc) {
                continue;
            }

            self.running.store(false, Ordering::Relaxed);
            self.break_reason = Some(bp.reason);
            return;
        }
    }

    pub fn run(&mut self) -> u64 {
        self.running.store(true, Ordering::Relaxed);
        let mut cycles_in_this_run = 0;
        while self.running.load(Ordering::Relaxed) {
            let start = Instant::now();
            let cycles = self.single_step();
            self.cycle_count += cycles;
            cycles_in_this_run += cycles;
            self.check_execute_breakpoints();
            let sleep_nanos = cycles * (1_000_000_000u64 / self.clock_speed);
            let end = Instant::now();
            std::thread::sleep(Duration::from_nanos(sleep_nanos).saturating_sub(end - start));
        }
        cycles_in_this_run
    }

    pub fn single_step(&mut self) -> u64 {
        let cycles = self.cpu.single_step(&mut self.memory_bus);

        let interrupts = self.tick_hardware(cycles);
        self.cpu.request_interrupts(&mut self.memory_bus, &interrupts);
        cycles
    }

    pub fn tick_hardware(&mut self, cycles: u64) -> InterruptRequest {
        let mut interrupts = InterruptRequest::default();

        interrupts.serial = self.memory_bus.serial.tick(cycles);
        let lcd = &mut self.memory_bus.lcd;
        let vblank_and_stat = self.memory_bus.ppu.tick(cycles, lcd);
        interrupts.vblank = vblank_and_stat.0;
        interrupts.stat = vblank_and_stat.1;
        interrupts.timer = self.memory_bus.timer_control.tick(cycles);

        // Update framebuffer on vblank
        if interrupts.vblank {
            let mut f = self.framebuffer.lock().unwrap();
            *f = *self.memory_bus.ppu.get_current_framebuffer();
        }

        interrupts
    }

    pub fn get_current_framebuffer(&self) -> [[Color; 160]; 144] {
        self.memory_bus.ppu.get_current_framebuffer().clone()
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
            address += insn.size() as u16;
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
        &self.memory_bus.cartridge
    }
}
