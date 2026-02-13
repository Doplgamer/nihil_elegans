use std::collections::HashMap;
use std::fmt::Write;
use color_eyre::eyre::eyre;
use reedline::ExternalPrinter;
use tokio::sync::mpsc::Sender;

pub fn print_banner(mode: &'static str) {
    let banner_top = vec![
        "\x1B[1m\x1B[31m\x1B[38;5;88m",
        "      .     .       .",
        "    o |   o |       |",
        ";-. . |-. . |   ,-. | ,-. ,-: ,-: ;-. ,-.",
        "| | | | | | |   |-' | |-' | | | | | | `-.",
        "' ' ' ' ' ' '   `-' ' `-' `-| `-` ' ' `-'",
    ];


    for line in banner_top {
        println!("{}", line);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    println!("\x1B[0m{}                    \x1B[1m\x1B[31m\x1B[38;5;88m`-'\x1B[0m{:>12}", mode, format!("v{}", env!("CARGO_PKG_VERSION")));
}

const B32_CHARSET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";


pub fn b32_encode(data: &[u8]) -> color_eyre::Result<Vec<u8>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let mut bin_data = String::with_capacity(data.len() * 8);
    for byte in data {
        write!(bin_data, "{:08b}", byte)?;
    }

    let rem = bin_data.len() % 5;
    if rem > 0 {
        for _ in 0..rem {
            bin_data.push('0');
        }
    }

    let mut result = Vec::new();
    for chunk in bin_data.as_bytes().chunks(5) {
        let chunk_str = std::str::from_utf8(chunk)?;
        let index = usize::from_str_radix(chunk_str, 2)?;
        result.push(B32_CHARSET[index])
    }

    while !result.len().is_multiple_of(8) {
        result.push(b'-')
    }

    Ok(result)
}

pub fn b32_decode(data: &[u8]) -> color_eyre::Result<Vec<u8>> {
    if data.is_empty() {
        return Ok(Vec::new());
    }

    let data_str = std::str::from_utf8(data).map_err(|_| eyre!("Invalid data in base32"))?
        .trim_end_matches("-");

    let mut bin_chunks = String::with_capacity(data_str.len() * 5);
    for ch in data_str.chars() {
        let index = B32_CHARSET
            .iter()
            .position(|&c| c == ch as u8)
            .ok_or_else(|| eyre!("Invalid base32 char: {ch}"))?;

        write!(bin_chunks, "{:05b}", index)?;
    }

    let mut result = Vec::new();
    for chunk in bin_chunks.as_bytes().chunks(8) {
        if chunk.len() == 8 {
            let chunk_str = std::str::from_utf8(chunk)?;
            let byte_value = u8::from_str_radix(chunk_str, 2)
                .map_err(|_| eyre!("Failed to parse chunk"))?;
            result.push(byte_value);
        }
    }

    Ok(result)
}

pub fn encrypt(data: &[u8], xor_key: &[u8]) -> color_eyre::Result<Vec<String>> {
    let mut b32_chunks: Vec<String> = Vec::new();
    let mut domain_names: Vec<String> = Vec::new();

    for chunk in data.chunks(35) {
        let encrypted = chunk.iter().enumerate().map(|(i, &b)| {
            b ^ xor_key[i % xor_key.len()]
        }).collect::<Vec<u8>>();
        let encoded = String::from_utf8(b32_encode(&encrypted)?)?;
        b32_chunks.push(format!("{}", encoded));
    }

    for name_chunk in b32_chunks.chunks(4) {
        domain_names.push(name_chunk.join("."))
    }

    Ok(domain_names)
}

pub fn decrypt(data: Vec<String>, xor_key: &[u8]) -> color_eyre::Result<Vec<u8>> {
    let mut b32_chunks: Vec<String> = Vec::new();
    let mut decrypted_data: Vec<u8> = Vec::new();

    for name in data {
        // Will eventually need to make this trim off the unimportant parts of the domain name
        for label_chunk in name.split('.').collect::<Vec<&str>>() {
            b32_chunks.push(label_chunk.to_string())
        }
    }

    for chunk in b32_chunks {
        let decoded = b32_decode(chunk.as_bytes())?;
        let mut decrypted = decoded.iter().enumerate().map(|(i, b)| {
            b ^ xor_key[i % xor_key.len()]
        }).collect::<Vec<u8>>();
        decrypted_data.append(&mut decrypted)
    }

    Ok(decrypted_data)
}

pub enum Action {
    Log(String),
    TempSend,
}

pub struct State {
    pub sender: Sender<Action>,

    pub commands: CommandMap,
    pub printer: ExternalPrinter<String>,
    pub is_ctrl_c_pressed: bool,
    pub exit: bool,
}

impl State {
    pub fn new(commands: CommandMap, printer: &ExternalPrinter<String>, sender: Sender<Action>) -> Self {
        Self {
            sender,

            commands,
            printer: printer.clone(),
            is_ctrl_c_pressed: false,
            exit: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Command {
    pub description: &'static str,
    pub function: fn(&mut State, &[&str]) -> color_eyre::Result<()>,
    is_subcommand: bool,
    has_subcommands: bool,
    subcommands: Option<Vec<Command>>,
}

impl Command {
    pub fn new(
        description: &'static str,
        function: fn(&mut State, &[&str]) -> color_eyre::Result<()>,
        is_subcommand: bool,
        has_subcommands: bool,
        subcommands: Option<Vec<Command>>) -> Self {
        Self {
            description,
            function,
            is_subcommand,
            has_subcommands,
            subcommands
        }
    }
}

pub type CommandMap = HashMap<&'static str, Command>;
