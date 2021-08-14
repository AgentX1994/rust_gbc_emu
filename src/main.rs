use clap::{App, Arg};
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::{event::Event, gfx::primitives::DrawRenderer, keyboard::Keycode, pixels::Color};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rust_gbc_emu::{
    debugger::Debugger,
    gbc::{mmio::lcd, Gbc},
};

fn run_debugger(gbc: Gbc) {
    let mut dbg = Debugger::new(gbc);
    dbg.run();
}

fn gbc_color_to_sdl_color(color: lcd::Color) -> Color {
    match color {
        lcd::Color::White => Color::RGBA(0xff, 0xff, 0xff, 0xff),
        lcd::Color::LightGray => Color::RGBA(0xaa, 0xaa, 0xaa, 0xff),
        lcd::Color::DarkGray => Color::RGBA(0x77, 0x77, 0x77, 0xff),
        lcd::Color::Black => Color::RGBA(0x00, 0x00, 0x00, 0xff),
    }
}

fn run(
    mut canvas: Canvas<Window>,
    mut event_pump: sdl2::EventPump,
    debugger_running: bool,
    framebuffer: Arc<Mutex<[[lcd::Color; 160]; 144]>>,
    gbc_running: Arc<AtomicBool>,
) {
    canvas.set_logical_size(160, 144).unwrap();
    canvas.clear();
    canvas.present();
    let frame_duration = Duration::from_nanos(1_000_000_000u64 / 60);
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    if debugger_running {
                        if gbc_running.load(Ordering::Relaxed) {
                            gbc_running.store(false, Ordering::Relaxed)
                        } else {
                            break 'running;
                        }
                    } else {
                        gbc_running.store(false, Ordering::Relaxed);
                        break 'running;
                    }
                }
                _ => {}
            }
        }

        let framebuffer = framebuffer.lock().unwrap();

        for row in 0..144u32 {
            for col in 0..160u32 {
                let color_gbc = framebuffer[row as usize][col as usize];
                // let bytes = color_u32.to_be_bytes();
                // let color = Color::RGBA(bytes[0], bytes[1], bytes[2], bytes[3]);
                let color = gbc_color_to_sdl_color(color_gbc);
                canvas.pixel(col as i16, row as i16, color).unwrap();
            }
        }

        canvas.present();
        ::std::thread::sleep(frame_duration);
    }
}

fn main() {
    let matches = App::new("rust_gbc_emu")
        .version("0.1.0")
        .author("John A. <johnasper94@gmail.com")
        .about("GB/GBC emulator")
        .arg(
            Arg::with_name("debug")
                .short("d")
                .long("debug")
                .help("Starts the debugger"),
        )
        .arg(
            Arg::with_name("instructions")
                .short("i")
                .long("instructions")
                .help("Shows each instruction as it's executed"),
        )
        .arg(Arg::with_name("ROM").required(true).index(1))
        .get_matches();

    let rom = matches.value_of("ROM").unwrap();
    let show_instructions = matches.is_present("instructions");
    let debug = matches.is_present("debug");

    let gbc_running = Arc::new(AtomicBool::new(false));
    let framebuffer = Arc::new(Mutex::new([[lcd::Color::White; 160]; 144]));
    let mut gbc = Gbc::new(
        rom,
        framebuffer.clone(),
        gbc_running.clone(),
        show_instructions,
    )
    .expect("Error Loading rom!");

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("Rust GBC Emu", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let canvas = window.into_canvas().build().unwrap();

    let event_sender = sdl_context.event().unwrap().event_sender();
    let start = Instant::now();
    let t = thread::spawn(move || {
        if debug {
            run_debugger(gbc);
            event_sender.push_event(Event::Quit{timestamp: (Instant::now() - start).as_millis() as u32}).unwrap();
        } else {
            gbc.run();
        }
    });

    let event_pump = sdl_context.event_pump().unwrap();
    run(canvas,event_pump, debug, framebuffer, gbc_running);

    t.join().expect("Error joining");
}
