use clap::{App, Arg};
use sdl2::render::Canvas;
use sdl2::video::Window;
use sdl2::{event::Event, keyboard::Keycode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use rust_gbc_emu::{
    debugger::Debugger,
    gbc::{mmio::lcd, Gbc, InputState},
};

fn run_debugger(gbc: Gbc) {
    let dbg = Debugger::new(gbc);
    dbg.run();
}

fn run(
    mut canvas: Canvas<Window>,
    mut event_pump: sdl2::EventPump,
    debugger_running: bool,
    framebuffer: &Arc<Mutex<[[lcd::Color; 160]; 144]>>,
    gbc_running: &Arc<AtomicBool>,
    input_state: &Arc<Mutex<InputState>>
) {
    canvas.set_logical_size(160, 144).unwrap();
    canvas.clear();
    canvas.present();
    let texture_creator = canvas.texture_creator();
    let mut texture = texture_creator
        .create_texture_streaming(canvas.default_pixel_format(), 160, 144)
        .unwrap();
    let format = texture.query().format;
    println!("Texture format: {:?}", format);
    let frame_duration = Duration::from_nanos(1_000_000_000_u64 / 60);
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
                            gbc_running.store(false, Ordering::Relaxed);
                        } else {
                            break 'running;
                        }
                    } else {
                        gbc_running.store(false, Ordering::Relaxed);
                        break 'running;
                    }
                }
                Event::KeyDown { keycode: Some(key), .. } => {
                    match key {
                        Keycode::Up => input_state.lock().unwrap().up_pressed = true,
                        Keycode::Down => input_state.lock().unwrap().down_pressed = true,
                        Keycode::Left => input_state.lock().unwrap().left_pressed = true,
                        Keycode::Right => input_state.lock().unwrap().right_pressed = true,
                        Keycode::Z => input_state.lock().unwrap().a_pressed = true,
                        Keycode::X => input_state.lock().unwrap().b_pressed = true,
                        Keycode::A => input_state.lock().unwrap().select_pressed = true,
                        Keycode::S => input_state.lock().unwrap().start_pressed = true,
                        _ => (),
                    }
                }
                Event::KeyUp { keycode: Some(key), .. } => {
                    match key {
                        Keycode::Up => input_state.lock().unwrap().up_pressed = false,
                        Keycode::Down => input_state.lock().unwrap().down_pressed = false,
                        Keycode::Left => input_state.lock().unwrap().left_pressed = false,
                        Keycode::Right => input_state.lock().unwrap().right_pressed = false,
                        Keycode::Z => input_state.lock().unwrap().a_pressed = false,
                        Keycode::X => input_state.lock().unwrap().b_pressed = false,
                        Keycode::A => input_state.lock().unwrap().select_pressed = false,
                        Keycode::S => input_state.lock().unwrap().start_pressed = false,
                        _ => (),
                    }
                }
                _ => {}
            }
        }

        let framebuffer = {
            let lock = framebuffer.lock().unwrap();
            *lock
        };

        // TODO other formats
        texture
            .with_lock(None, |data, pitch| {
                let mut row_i = 0;
                for row in framebuffer {
                    let mut i = row_i;
                    for pixel in row {
                        let bytes = match pixel {
                            lcd::Color::White => [0xff, 0xff, 0xff],
                            lcd::Color::LightGray => [0xaa, 0xaa, 0xaa],
                            lcd::Color::DarkGray => [0x77, 0x77, 0x77],
                            lcd::Color::Black => [0x00, 0x00, 0x00],
                        };
                        data[i] = bytes[0];
                        data[i + 1] = bytes[1];
                        data[i + 2] = bytes[2];
                        i += 4;
                    }
                    row_i += pitch;
                }
            })
            .unwrap();

        // for row in 0..144u32 {
        //     for col in 0..160u32 {
        //         let color_gbc = framebuffer[row as usize][col as usize];
        //         // let bytes = color_u32.to_be_bytes();
        //         // let color = Color::RGBA(bytes[0], bytes[1], bytes[2], bytes[3]);
        //         let color = gbc_color_to_sdl_color(color_gbc);
        //         canvas.pixel(col as i16, row as i16, color).unwrap();
        //     }
        // }

        canvas.copy(&texture, None, None).unwrap();
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
        .arg(
            Arg::with_name("turbo")
                .short("t")
                .long("turbo")
                .help("Removes limits on run speed"),
        )
        .arg(Arg::with_name("ROM").required(true).index(1))
        .get_matches();

    let rom = matches.value_of("ROM").unwrap().to_string();
    let show_instructions = matches.is_present("instructions");
    let debug = matches.is_present("debug");
    let turbo = matches.is_present("turbo");

    let gbc_running = Arc::new(AtomicBool::new(false));
    let framebuffer = Arc::new(Mutex::new([[lcd::Color::White; 160]; 144]));
    let input_state = Arc::new(Mutex::new(InputState::default()));

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

    let gbc_running_gbc = gbc_running.clone();
    let framebuffer_gbc = framebuffer.clone();
    let input_state_gbc = input_state.clone();
    let t = thread::spawn(move || {
        let mut gbc = Gbc::new(
            rom,
            framebuffer_gbc,
            gbc_running_gbc,
            turbo,
            show_instructions,
            input_state_gbc
        )
        .expect("Error Loading rom!");
        if debug {
            run_debugger(gbc);
            #[allow(clippy::cast_possible_truncation)]
            event_sender
                .push_event(Event::Quit {
                    timestamp: (Instant::now() - start).as_millis() as u32,
                })
                .unwrap();
        } else {
            let start = Instant::now();
            let (cycles, encountered_problem) = gbc.run();
            if encountered_problem {
                println!("Encountered an unknown instruction, halting!");
            }
            let runtime = Instant::now() - start;
            let cpu_speed = gbc.get_clock_speed();
            #[allow(clippy::cast_precision_loss)]
            let actual_clock_speed = cycles as f64 / runtime.as_secs_f64();
            #[allow(clippy::cast_precision_loss)]
            let percentage_speed = 100.0 * (actual_clock_speed / cpu_speed as f64);
            println!(
                "{} cycles in {:.02} - {:>10.02}hz ({:.02}%)",
                cycles,
                runtime.as_secs_f64(),
                actual_clock_speed,
                percentage_speed
            );
        }
    });

    let event_pump = sdl_context.event_pump().unwrap();
    run(canvas, event_pump, debug, &framebuffer, &gbc_running, &input_state);

    t.join().expect("Error joining");
}
