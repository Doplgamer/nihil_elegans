use std::net::{IpAddr, SocketAddr};
use clap::Parser;
use hickory_server::ServerFuture;
use inquire::Confirm;
use reedline::{ExternalPrinter, Reedline, Signal};
use shared::{print_banner, Action, State};

use color_eyre::Result;
use tokio::select;
use crate::{
    handler::MyHandler,
    commands::init_commands,
    prompt::NihilPrompt,
};

mod commands;
mod handler;
mod prompt;

#[derive(Parser)]
pub struct CliArgs {
    /// Host Address
    #[arg(short, long, value_parser, default_value = "127.0.0.1")]
    address: IpAddr,
    /// Listening Port (Run with root access to use port 53)
    #[arg(short, long, value_parser, default_value = "5053")]
    port: u16,
    /// XOR Key used to encrypt data
    #[arg(long, default_value = "sisyphean")]
    xor_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli_args = CliArgs::parse();

    let (sender, mut receiver) = tokio::sync::mpsc::channel(100);

    let commands = init_commands();
    let printer = ExternalPrinter::new(32);
    let event_printer = printer.clone();

    let app_state = State::new(commands, &printer, sender.clone());

    let handler = MyHandler::new(app_state.sender.clone(), cli_args.xor_key);
    let mut server = ServerFuture::new(handler);

    let server_addr = SocketAddr::new(
        cli_args.address,
        cli_args.port,
    );

    let socket = tokio::net::UdpSocket::bind(server_addr).await?;
    server.register_socket(socket);

    let mut line_editor = Reedline::create()
        .with_external_printer(printer);
    let mut prompt = NihilPrompt::new(app_state);

    print_banner("Server");
    println!("Up and running at: {}", server_addr);

    let reedline_handle = tokio::task::spawn_blocking(move || -> Result<()> {
        while !prompt.state.exit {
            if prompt.state.is_ctrl_c_pressed {
                prompt.state.is_ctrl_c_pressed = false;
                let input = Confirm::new("Ctrl+C was pressed; would you like to exit?")
                    .with_default(false)
                    .prompt()?;
                if input {
                    prompt.state.exit = true;
                }
                continue;
            }

            let sig = line_editor.read_line(&prompt);
            match sig {
                Ok(Signal::Success(buffer)) => {
                    let args = buffer.split_whitespace().collect::<Vec<&str>>();
                    if !args.is_empty() {
                        if let Some(cmd) = prompt.state.commands.get(args[0]) {
                            (cmd.function)(&mut prompt.state, &args[1..])?
                        }
                    }
                    prompt.state.printer.print(format!("Processed: {}", buffer).into())?;
                },
                Ok(Signal::CtrlD) => {
                    prompt.state.printer.print("\nAborted!".into())?;
                    break;
                },
                Ok(Signal::CtrlC) => {
                    prompt.state.is_ctrl_c_pressed = true;
                },
                x => {
                    // prompt.state.printer.print(format!("Unknown event: {:?}", x).into())?;
                    println!("Unknown event: {:?}", x);
                }
            }
        }

        Ok(())
    });

    let event_handle = tokio::task::spawn(async move {
        loop {
            select! {
                maybe_event = receiver.recv() => {
                    match maybe_event {
                        Some(event) => {
                            match event {
                                Action::Log(msg) => {
                                    event_printer.print(msg).unwrap();
                                }
                                Action::TempSend => {}
                            }
                        },
                        _ => break,
                    }
                }
            }
        }
    });


    reedline_handle.await??;
    event_handle.abort();

    Ok(())
}