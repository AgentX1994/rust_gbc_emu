use std::{fmt, fmt::Display};

use rustyline::{error::ReadlineError, Editor};

use crate::gbc::{Gbc, debug::{AccessType, BreakReason}, ppu::TileAddressingMethod};

use parse_int::parse;

#[derive(Debug)]
enum TokenizerError {
    UnmatchedQuote(char),
}

impl Display for TokenizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnmatchedQuote(quote) => write!(f, "Unmatched {} character", quote),
        }
    }
}

fn tokenize_line(line: &str) -> Result<Vec<String>, TokenizerError> {
    let mut tokens = vec![];
    let mut cur_token = String::new();
    let mut saw_quote = false;
    let mut quote_type = '"';
    for c in line.chars() {
        if saw_quote {
            if c == quote_type {
                saw_quote = false;
            } else {
                cur_token.push(c);
            }
        } else if c == '\'' || c == '"' {
            saw_quote = true;
            quote_type = c;
        } else if c == ' ' {
            tokens.push(cur_token);
            cur_token = String::new();
        } else {
            cur_token.push(c);
        }
    }
    if saw_quote {
        return Err(TokenizerError::UnmatchedQuote(quote_type));
    }
    if !cur_token.is_empty() {
        tokens.push(cur_token);
    }

    Ok(tokens)
}

enum Command {
    Unknown,
    Exit,
    Reset,
    DumpState,
    AddBreakpoint,
    ListBreakpoints,
    DeleteBreakpoint,
    Run,
    Step,
    Read,
    Disassemble,
    PrintHeaderDetails,
    DumpTileMap,
    DumpTiles,
    DumpSprites,
}

impl Command {
    fn from_string(input: String) -> Command {
        match input.as_str() {
            "exit" | "q" | "quit" => Command::Exit,
            "restart" | "reset" => Command::Reset,
            "state" | "dump" | "regs" => Command::DumpState,
            "break" | "breakpoint" | "b" | "bp" => Command::AddBreakpoint,
            "list" | "bl" | "lb" | "listbreak" => Command::ListBreakpoints,
            "bc" | "delete" | "del" | "clear" | "clearbreak" | "cb" => Command::DeleteBreakpoint,
            "r" | "run" | "g" | "go" => Command::Run,
            "s" | "step" | "n" | "next" => Command::Step,
            "p" | "print" | "read" | "readmem" => Command::Read,
            "disassemble" | "dis" | "disass" | "u" => Command::Disassemble,
            "header" => Command::PrintHeaderDetails,
            "tilemap" => Command::DumpTileMap,
            "tiles" => Command::DumpTiles,
            "sprites" => Command::DumpSprites,
            _ => Command::Unknown,
        }
    }
}

const LINE_LENGTH: u16 = 4;

pub struct Debugger {
    gbc: Gbc,
}

impl Debugger {
    #[must_use]
    pub fn new(gbc: Gbc) -> Self {
        Debugger { gbc }
    }

    pub fn run(mut self) {
        let mut rl = Editor::<()>::new();
        if rl.load_history("history.txt").is_err() {
            // Do nothing
        }
        loop {
            let readline = rl.readline(">> ");
            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str());
                    let tokens = match tokenize_line(line.as_str()) {
                        Ok(tokens) => {
                            if tokens.is_empty() {
                                match rl.history().last() {
                                    Some(l) => match tokenize_line(l.as_str()) {
                                        Ok(tokens) => tokens,
                                        Err(_) => continue,
                                    },
                                    None => continue,
                                }
                            } else {
                                tokens
                            }
                        }
                        Err(e) => {
                            println!("Error: {}", e);
                            continue;
                        }
                    };

                    let should_continue = match Command::from_string(tokens[0].to_lowercase()) {
                        Command::Exit => self.run_command_exit(&tokens[..]),
                        Command::Reset => self.run_command_reset(&tokens[..]),
                        Command::DumpState => self.run_command_dump_state(&tokens[..]),
                        Command::AddBreakpoint => self.run_command_add_breakpoint(&tokens[..]),
                        Command::ListBreakpoints => self.run_command_list_breakpoints(&tokens[..]),
                        Command::DeleteBreakpoint => {
                            self.run_command_delete_breakpoint(&tokens[..])
                        }
                        Command::Run => self.run_command_run(&tokens[..]),
                        Command::Step => self.run_command_step(&tokens[..]),
                        Command::Read => self.run_command_read(&tokens[..]),
                        Command::Disassemble => self.run_command_disassemble(&tokens[..]),
                        Command::PrintHeaderDetails => {
                            self.run_command_print_header_details(&tokens[..])
                        }
                        Command::DumpTileMap => self.run_command_dump_tile_map(&tokens[..]),
                        Command::DumpTiles => self.run_command_dump_tiles(&tokens[..]),
                        Command::DumpSprites => self.run_command_dump_sprites(&tokens[..]),
                        Command::Unknown => {
                            println!("Unknown command {}", tokens[0]);
                            true
                        }
                    };

                    if !should_continue {
                        break;
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("CTRL-C");
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    break;
                }
                Err(err) => {
                    println!("Error: {:?}", err);
                    break;
                }
            }
        }
        rl.save_history("history.txt").unwrap();
    }

    fn run_command_exit(&mut self, _args: &[String]) -> bool {
        println!("Exiting");
        false
    }

    fn run_command_reset(&mut self, _args: &[String]) -> bool {
        self.gbc.reset();

        true
    }

    fn run_command_dump_state(&mut self, _args: &[String]) -> bool {
        self.gbc.dump_state();

        true
    }

    fn run_command_add_breakpoint(&mut self, args: &[String]) -> bool {
        if args.len() < 2 {
            println!("Usage: {} <address> [access type] [length]", args[0]);
            return true;
        }
        let address = match parse(args[1].as_str()) {
            Ok(address) => address,
            Err(e) => {
                println!("Error: invalid address: {}", e);
                return true;
            }
        };
        let access_type = if args.len() > 2 {
            match AccessType::from_string(args[2].as_str()) {
                Ok(a) => a,
                Err(e) => {
                    println!("Error: invalid access type: {}", e);
                    return true;
                }
            }
        } else {
            AccessType::Execute
        };
        let length = if args.len() > 3 {
            match parse(args[3].as_str()) {
                Ok(length) => length,
                Err(e) => {
                    println!("Error: invalid length: {}", e);
                    return true;
                }
            }
        } else {
            1
        };
        self.gbc
            .add_breakpoint(address, access_type, length, BreakReason::User);

        true
    }

    fn run_command_list_breakpoints(&mut self, _args: &[String]) -> bool {
        let breakpoints = self.gbc.list_breakpoints();
        if breakpoints.is_empty() {
            println!("No breakpoints");
        } else {
            for (i, bp) in breakpoints.iter().enumerate() {
                println!(
                    "Breakpiont {}: {:04x} {} {} bytes {}",
                    i, bp.address, bp.access_type, bp.length, bp.reason
                );
            }
        }

        true
    }

    fn run_command_delete_breakpoint(&mut self, args: &[String]) -> bool {
        let mut indices: Vec<usize> = Vec::new();
        for index_str in args[1..].iter().flat_map(|i| i.split(',')) {
            match parse(index_str) {
                Ok(index) => indices.push(index),
                Err(e) => println!("Invalid index {}: {}", index_str, e),
            }
        }

        // TODO remove breakpoints

        true
    }

    fn run_command_run(&mut self, _args: &[String]) -> bool {
        let (_, problem) = self.gbc.run();
        if problem {
            println!("Encountered an unknown instruction!");
        } else {
            match self.gbc.get_last_breakpoint() {
                Some(bp) => println!("Break Reason: {}", bp),
                None => println!("Break Reason: None"),
            }
        }
        self.gbc.print_next_instruction();

        true
    }

    fn run_command_step(&mut self, _args: &[String]) -> bool {
        self.gbc.single_step();
        self.gbc.print_next_instruction();
        self.gbc.dump_cpu_state();

        true
    }

    fn run_command_read(&mut self, args: &[String]) -> bool {
        if args.len() < 2 {
            println!("Usage: {} <address> [length]", args[0]);
            return true;
        }
        let address = match parse(args[1].as_str()) {
            Ok(address) => address,
            Err(e) => {
                println!("Error: invalid address: {}", e);
                return true;
            }
        };
        let length = if args.len() > 2 {
            match parse(args[2].as_str()) {
                Ok(length) => length,
                Err(e) => {
                    println!("Error: invalid length: {}", e);
                    return true;
                }
            }
        } else {
            16
        };

        let bytes = self.gbc.read_memory(address, length);
        let mut cur_addr = address;
        for chunk in bytes.chunks(LINE_LENGTH as usize) {
            print!("{:04x}: ", cur_addr);
            for byte in chunk {
                print!("{:02x} ", byte);
            }
            for byte in chunk {
                if let 0x20..=0x7e = byte {
                    print!("{}", *byte as char);
                } else {
                    print!(".");
                }
            }
            println!();
            cur_addr += LINE_LENGTH as u16;
        }

        true
    }

    fn run_command_disassemble(&mut self, args: &[String]) -> bool {
        let address = if args.len() > 1 {
            match parse(args[1].as_str()) {
                Ok(address) => Some(address),
                Err(e) => {
                    println!("Error: invalid address: {}", e);
                    return true;
                }
            }
        } else {
            None
        };
        let length = if args.len() > 2 {
            match parse(args[2].as_str()) {
                Ok(length) => length,
                Err(e) => {
                    println!("Error: invalid length: {}", e);
                    return true;
                }
            }
        } else {
            16
        };

        self.gbc.print_instructions(address, length);

        true
    }

    fn run_command_print_header_details(&mut self, _args: &[String]) -> bool {
        let cart = self.gbc.get_cartridge();
        println!("Cartridge: {}", cart.title);
        println!("\tManufacturer Code: {:?}", cart.manufacturer_code);
        println!(
            "\tLicensee Code: {} {}",
            cart.licensee_code[0] as char, cart.licensee_code[1] as char
        );
        println!("\tCartridge Type: {:?}", cart.cartridge_type);
        println!("\tColor Support: {:?}", cart.color_support);
        println!("\tSGB Support: {}", cart.supports_sgb);
        println!(
            "\tROM Size: {} ({} banks)",
            cart.rom_size,
            cart.rom_size / 16384
        );
        println!(
            "\tRAM Size: {} ({} banks)",
            cart.external_ram_size,
            cart.external_ram_size / 8192
        );
        println!("\tIs Japanese: {}", cart.is_japanese);
        println!("\tROM Version: {}", cart.rom_version);
        println!(
            "\tExternal RAM currently enabled: {}",
            cart.enable_external_ram
        );
        println!("\tCurrently selected ROM bank: {}", cart.rom_bank_selected);
        println!(
            "\tCurrent banking mode: {}",
            if cart.advanced_banking_mode {
                "advanced"
            } else {
                "basic"
            }
        );

        return true;
    }

    fn run_command_dump_tile_map(&mut self, args: &[String]) -> bool {
        if args.len() != 2 {
            println!("Usage: {} <tilemap number, 0 or 1>", args[0]);
            return true;
        }

        let map = match parse(args[1].as_str()) {
            Ok(m) => m,
            Err(e) => {
                println!("Error: invalid number: {}", e);
                return true;
            }
        };

        let tilemap = match self.gbc.get_tile_map(map) {
            Some(m) => m,
            None => {
                println!("Error: no tilemap {}, only 0 or 1", map);
                return true;
            }
        };

        // let show_color = args.len() == 3 && args[2] == "color";

        for tiles in tilemap.chunks(32) {
            for tile in tiles {
                print!("{:02x} ", tile);
            }
            println!();
        }

        true
    }

    fn run_command_dump_tiles(&mut self, args: &[String]) -> bool {
        if args.len() != 3 {
            println!("Usage: {} <tile index> <indexing method: 8000 | 9000", args[0]);
            return true;
        }

        let tile_index: i16 = match parse(args[1].as_str()) {
            Ok(i) => i,
            Err(e) => {
                println!("Error: invalid tile_index: {}", e);
                return true;
            }
        };

        let tile_address = match args[2].as_str() {
            "8000" => {
                if tile_index < 0 || tile_index > 255 {
                    println!("Error: tile index {} out of range for From8000 addressing, should be 0 <= tile index < 256", tile_index);
                    return true;
                }
                TileAddressingMethod::From8000(tile_index as u8)
            }
            "9000" => {
                if tile_index < -128 || tile_index > 127 {
                    println!("Error: tile index {} out of range for From9000 addressing, should be -128 <= tile index < 128", tile_index);
                    return true;
                }
                TileAddressingMethod::From9000(tile_index as i8)
            }
            _ => {
                println!("Error: unknown addressing method {}", args[2]);
                return true;
            }
        };

        let tile = self.gbc.get_tile(tile_address);

        for row in (&tile.deinterleave()[..]).chunks(8) {
            for &color in row {
                let c: u8 = color.into();
                print!("{}", c);
            }
            println!();
        }

        true
    }

    fn run_command_dump_sprites(&mut self, _args: &[String]) -> bool {
        todo!();
    }
}
