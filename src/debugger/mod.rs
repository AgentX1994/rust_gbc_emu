use std::{fmt, fmt::Display, num::ParseIntError};

use rustyline::{error::ReadlineError, Editor};

use crate::gbc::{
    debug::{AccessType, BreakReason},
    Gbc,
};

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
    if cur_token != "" {
        tokens.push(cur_token);
    }

    Ok(tokens)
}

fn parse_u16_hex(s: &str) -> Result<u16, ParseIntError> {
    let without_prefix = s.trim_start_matches("0x");
    u16::from_str_radix(without_prefix, 16)
}

#[derive(Debug)]
pub struct Debugger {
    gbc: Gbc,
}

impl Debugger {
    pub fn new(gbc: Gbc) -> Self {
        Debugger { gbc }
    }

    pub fn run(&mut self) {
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
                            if tokens.len() == 0 {
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

                    match tokens[0].to_lowercase().as_str() {
                        "exit" | "q" | "quit" => {
                            println!("Exiting...");
                            break;
                        }
                        "break" | "breakpoint" | "b" | "bp" => {
                            if tokens.len() < 2 {
                                println!("Usage: {} <address> [access type] [length]", tokens[0]);
                                continue;
                            }
                            let address = match parse_u16_hex(tokens[1].as_str()) {
                                Ok(address) => address,
                                Err(e) => {
                                    println!("Error: invalid address: {}", e);
                                    continue;
                                }
                            };
                            let access_type = if tokens.len() > 2 {
                                match AccessType::from_string(tokens[2].as_str()) {
                                    Ok(a) => a,
                                    Err(e) => {
                                        println!("Error: invalid access type: {}", e);
                                        continue;
                                    }
                                }
                            } else {
                                AccessType::Execute
                            };
                            let length = if tokens.len() > 3 {
                                match tokens[3].parse::<u16>() {
                                    Ok(length) => length,
                                    Err(e) => {
                                        println!("Error: invalid length: {}", e);
                                        continue;
                                    }
                                }
                            } else {
                                1
                            };
                            self.gbc.add_breakpoint(
                                address,
                                access_type,
                                length,
                                BreakReason::User,
                            );
                        }
                        "list" | "bl" | "lb" | "listbreak" => {
                            let breakpoints = self.gbc.list_breakpoints();
                            if breakpoints.len() == 0 {
                                println!("No breakpoints");
                            } else {
                                for (i, bp) in breakpoints.iter().enumerate() {
                                    println!(
                                        "Breakpiont {}: {:04x} {} {} bytes {}",
                                        i, bp.address, bp.access_type, bp.length, bp.reason
                                    );
                                }
                            }
                        }
                        "bc" | "delete" | "del" | "clear" | "clearbreak" | "cb" => {
                            for index_str in tokens[1..].iter().flat_map(|i| i.split(",")) {
                                match index_str.parse::<usize>() {
                                    Ok(index) => self.gbc.remove_breakpoint(index),
                                    Err(e) => println!("Invalid index {}: {}", index_str, e),
                                }
                            }
                        }
                        "r" | "run" | "g" | "go" => {
                            self.gbc.run();
                            self.gbc.print_next_instruction();
                        }
                        "state" | "dump" | "regs" => {
                            self.gbc.dump_state();
                        }
                        "s" | "step" | "n" | "next" => {
                            self.gbc.single_step();
                            self.gbc.print_next_instruction();
                            self.gbc.dump_cpu_state();
                        }
                        "p" | "print" | "read" | "readmem" => {
                            if tokens.len() < 2 {
                                println!("Usage: {} <address> [length]", tokens[0]);
                                continue;
                            }
                            let address = match parse_u16_hex(tokens[1].as_str()) {
                                Ok(address) => address,
                                Err(e) => {
                                    println!("Error: invalid address: {}", e);
                                    continue;
                                }
                            };
                            let length = if tokens.len() > 2 {
                                match tokens[2].parse::<u16>() {
                                    Ok(length) => length,
                                    Err(e) => {
                                        println!("Error: invalid length: {}", e);
                                        continue;
                                    }
                                }
                            } else {
                                16
                            };

                            let bytes = self.gbc.read_memory(address, length);
                            const LINE_LENGTH: usize = 4;
                            let mut cur_addr = address;
                            for chunk in bytes.chunks(LINE_LENGTH) {
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
                        }
                        "disassemble" | "dis" | "disass" | "u" => {
                            let address = if tokens.len() > 1 {
                                match parse_u16_hex(tokens[1].as_str()) {
                                    Ok(address) => Some(address),
                                    Err(e) => {
                                        println!("Error: invalid address: {}", e);
                                        continue;
                                    }
                                }
                            } else {
                                None
                            };
                            let length = if tokens.len() > 2 {
                                match tokens[2].parse::<u16>() {
                                    Ok(length) => length,
                                    Err(e) => {
                                        println!("Error: invalid length: {}", e);
                                        continue;
                                    }
                                }
                            } else {
                                16
                            };

                            self.gbc.print_instructions(address, length);
                        }
                        "header" => {
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
                                cart.ram_size,
                                cart.ram_size / 8192
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
                        }
                        _ => println!("Unknown command {}", tokens[0]),
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
}
