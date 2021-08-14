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
    cartridge: Arc<Mutex<Cartridge>>,
    ram: Arc<Mutex<[u8; 8192]>>,
    ppu: Arc<Mutex<PictureProcessingUnit>>,
    joypad: Arc<Mutex<Joypad>>,
    serial: Arc<Mutex<SerialComms>>,
    timer_control: Arc<Mutex<Timer>>,
    sound: Arc<Mutex<Sound>>,
    lcd: Arc<Mutex<Lcd>>,
    vram_select: Arc<Mutex<u8>>,
    disable_boot_rom: Arc<Mutex<bool>>,
    vram_dma: Arc<Mutex<[u8; 4]>>,
    color_palettes: Arc<Mutex<[u8; 2]>>,
    wram_bank_select: Arc<Mutex<u8>>,
    interrupt_flags: Arc<Mutex<u8>>,
    high_ram: Arc<Mutex<[u8; 127]>>,
    interrupt_enable: Arc<Mutex<u8>>,
    cycle_count: u64,
    breakpoints: Vec<Breakpoint>,
    break_reason: Option<BreakReason>,
}

impl Gbc {
    pub fn new<P: AsRef<Path>>(
        rom_path: P,
        framebuffer: Arc<Mutex<[[Color; 160]; 144]>>,
        running: Arc<AtomicBool>,
        show_instructions: bool,
    ) -> io::Result<Self> {
        let cartridge = Cartridge::new(rom_path)?;
        Ok(Gbc {
            running,
            framebuffer,
            clock_speed: 4194304, // TODO switch based on detected cartridge / config
            cpu: Cpu::new(show_instructions),
            cartridge: Arc::new(Mutex::new(cartridge)),
            ram: Arc::new(Mutex::new([0; 8192])),
            ppu: Arc::new(Mutex::new(PictureProcessingUnit::default())),
            joypad: Arc::new(Mutex::new(Joypad::default())),
            serial: Arc::new(Mutex::new(SerialComms::default())),
            timer_control: Arc::new(Mutex::new(Timer::default())),
            sound: Arc::new(Mutex::new(Sound::default())),
            lcd: Arc::new(Mutex::new(Lcd::default())),
            vram_select: Arc::new(Mutex::new(0)),
            disable_boot_rom: Arc::new(Mutex::new(false)),
            vram_dma: Arc::new(Mutex::new([0; 4])),
            color_palettes: Arc::new(Mutex::new([0; 2])),
            wram_bank_select: Arc::new(Mutex::new(0)),
            interrupt_flags: Arc::new(Mutex::new(0)),
            high_ram: Arc::new(Mutex::new([0; 127])),
            interrupt_enable: Arc::new(Mutex::new(0)),
            cycle_count: 0,
            breakpoints: Vec::new(),
            break_reason: None,
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

    fn create_memory_bus(&self) -> MemoryBus {
        MemoryBus::new(
            self.cartridge.clone(),
            self.ram.clone(),
            self.ppu.clone(),
            self.joypad.clone(),
            self.serial.clone(),
            self.timer_control.clone(),
            self.sound.clone(),
            self.lcd.clone(),
            self.vram_select.clone(),
            self.disable_boot_rom.clone(),
            self.vram_dma.clone(),
            self.color_palettes.clone(),
            self.wram_bank_select.clone(),
            self.interrupt_flags.clone(),
            self.high_ram.clone(),
            self.interrupt_enable.clone(),
        )
    }

    pub fn run(&mut self) {
        self.running.store(true, Ordering::Relaxed);
        while self.running.load(Ordering::Relaxed) {
            let start = Instant::now();
            let cycles = self.single_step();
            self.cycle_count += cycles;
            self.check_execute_breakpoints();
            let sleep_nanos = cycles * (1_000_000_000u64 / self.clock_speed);
            let end = Instant::now();
            std::thread::sleep(Duration::from_nanos(sleep_nanos).saturating_sub(end - start));
        }
    }

    pub fn single_step(&mut self) -> u64 {
        let mut memory_bus = self.create_memory_bus();
        let cycles = self.cpu.single_step(&mut memory_bus);

        let interrupts = self.tick_hardware(cycles);
        self.cpu.request_interrupts(&mut memory_bus, &interrupts);
        cycles
    }

    pub fn tick_hardware(&mut self, cycles: u64) -> InterruptRequest {
        let mut interrupts = InterruptRequest::default();

        interrupts.serial = self.serial.lock().unwrap().tick(cycles);
        let mut lcd = self.lcd.lock().unwrap();
        let vblank_and_stat = self.ppu.lock().unwrap().tick(cycles, &mut *&mut lcd);
        interrupts.vblank = vblank_and_stat.0;
        interrupts.stat = vblank_and_stat.1;
        interrupts.timer = self.timer_control.lock().unwrap().tick(cycles);

        // Update framebuffer on vblank
        if interrupts.vblank {
            let mut f = self.framebuffer.lock().unwrap();
            *f = *self.ppu.lock().unwrap().get_current_framebuffer();
        }

        interrupts
    }

    pub fn get_current_framebuffer(&self) -> [[Color; 160]; 144] {
        self.ppu.lock().unwrap().get_current_framebuffer().clone()
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

        let memory_bus = self.create_memory_bus();
        for _ in 0..length {
            let insn = self.cpu.get_instruction_at_address(&memory_bus, address);
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

    pub fn get_cartridge(&self) -> Arc<Mutex<Cartridge>> {
        self.cartridge.clone()
    }
}
