use clap::{App, Arg};
use rust_gbc_emu::{gbc::Gbc, debugger::Debugger};

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
            .help("Shows each instruction as it's executed")
        )
        .arg(Arg::with_name("ROM").required(true).index(1))
        .get_matches();

    let rom = matches.value_of("ROM").unwrap();
    let mut gbc = Gbc::new(rom, matches.is_present("instructions")).expect("Error Loading rom!");
    if matches.is_present("debug") {
        let mut dbg = Debugger::new(gbc);
        dbg.run();
    } else {
        //gbc.dump_state();
        //println!();
        gbc.run();
        //println!();
        //gbc.dump_state();
    }
}
