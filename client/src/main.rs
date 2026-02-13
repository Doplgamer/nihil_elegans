use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use clap::Parser;
use hickory_client::{
    client::{Client, ClientHandle},
    proto::{
        rr::{DNSClass, IntoName, RecordType},
        runtime::TokioRuntimeProvider,
        udp::UdpClientStream,
    },
};
use color_eyre::Result;
use inquire::Confirm;
use reedline::{ExternalPrinter, Reedline, Signal};
use tokio::select;
use tokio::sync::{mpsc::Sender, Mutex};
use shared::{encrypt, print_banner, Action, State};
use crate::{
    commands::init_commands,
    prompt::NihilPrompt,
};

mod commands;
mod prompt;

#[derive(Parser)]
pub struct CliArgs {
    /// Target Address
    #[arg(short, long, value_parser, default_value = "127.0.0.1")]
    address: IpAddr,
    /// Target Port
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

    let addr = SocketAddr::new(
        cli_args.address,
        cli_args.port,
    );
    let conn = UdpClientStream::builder(addr, TokioRuntimeProvider::default()).build();

    let mut line_editor = Reedline::create()
        .with_external_printer(printer);
    let mut prompt = NihilPrompt::new(app_state);

    print_banner("Client");
    // TODO make DNS it's own command (with subcommands)

    let (client, bg) = Client::connect(conn).await?;
    tokio::spawn(bg);

    let client = Arc::new(Mutex::new(client));

    prompt.state.printer.print(format!("Ready and waiting to shoot queries at: {}", addr).into())?;

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
                }
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
                                    if let Err(e) = event_printer.print(msg) {
                                        eprintln!("Failed to print log message: {}", e);
                                    }
                                }
                                Action::TempSend => {
                                    event_printer.print("Sending test message...".into()).ok();

                                    let client_clone = client.clone();
                                    let sender_clone = sender.clone();
                                    let printer_clone = event_printer.clone();

                                    let xor_key_clone = cli_args.xor_key.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = send_test_message(client_clone, sender_clone, xor_key_clone).await {
                                            let error_msg = format!("Error sending test message: {}", e);
                                            printer_clone.print(error_msg.into()).ok();
                                        } else {
                                            printer_clone.print("Test message sent successfully!".into()).ok();
                                        }
                                    });
                                }
                            }
                        },
                        None => {
                            event_printer.print("Channel closed, shutting down event handler".into()).ok();
                            break;
                        },
                    }
                }
            }
        }
    });

    reedline_handle.await.map_err(|e| color_eyre::eyre::eyre!("Reedline task failed: {}", e))??;
    event_handle.abort();

    Ok(())
}

async fn send_test_message(client: Arc<Mutex<Client>>, sender: Sender<Action>, xor_key: String) -> Result<()> {
    let plaintext = "\"Did you ever hear the tragedy of Darth Plagueis The Wise? I thought not. It’s not a story the Jedi would tell you. It’s a Sith legend. Darth Plagueis was a Dark Lord of the Sith, so powerful and so wise he could use the Force to influence the midichlorians to create life… He had such a knowledge of the dark side that he could even keep the ones he cared about from dying. The dark side of the Force is a pathway to many abilities some consider to be unnatural. He became so powerful… the only thing he was afraid of was losing his power, which eventually, of course, he did. Unfortunately, he taught his apprentice everything he knew, then his apprentice killed him in his sleep. Ironic. He could save others from death, but not himself.\" - Darth Sidious.";

    let b32_data = encrypt(plaintext.as_bytes(), xor_key.as_bytes())?;

    sender.send(Action::Log(format!("Sending {} DNS queries...", b32_data.len()))).await.ok();

    for (index, name) in b32_data.iter().enumerate() {
        let domain_name = format!("{}", str::from_utf8(&name.as_bytes())?);
        sender.send(Action::Log(format!("[{}/{}] Querying: {}", index + 1, b32_data.len(), domain_name))).await.ok();

        let mut client_guard = client.lock().await;
        let response = client_guard.query(
            domain_name.into_name()?,
            DNSClass::IN,
            RecordType::A,
        ).await?;

        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        if !response.answers().is_empty() {
            sender.send(Action::Log(
                format!("[{}/{}] Server echoed: {}",
                        index + 1,
                        b32_data.len(),
                        response.answers()[0].to_string().to_uppercase()
                )
            )).await.ok();
        } else {
            sender.send(Action::Log(
                format!("[{}/{}] No answer from server", index + 1, b32_data.len())
            )).await.ok();
        }
    }

    sender.send(Action::Log("All queries completed.".into())).await.ok();

    Ok(())
}