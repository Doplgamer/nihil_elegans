use std::net::SocketAddr;
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
use tokio::sync::mpsc::Sender;
use shared::{encrypt, print_banner, Action, State};
use crate::{
    commands::init_commands,
    prompt::NihilPrompt,
};

mod commands;
mod prompt;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let (sender, mut receiver) = tokio::sync::mpsc::channel(100);

    let commands = init_commands();
    let printer = ExternalPrinter::new(32);
    let event_printer = printer.clone();

    let app_state = State::new(commands, &printer, sender.clone());

    let addr: SocketAddr = "127.0.0.1:5053".parse()?;
    let conn = UdpClientStream::builder(addr, TokioRuntimeProvider::default()).build();

    let mut line_editor = Reedline::create()
        .with_external_printer(printer);
    let mut prompt = NihilPrompt::new(app_state);

    print_banner("Client");
    // TODO make DNS it's own command (with subcommands)

    let (mut client, bg) = Client::connect(conn).await?;
    tokio::spawn(bg);
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
                                    event_printer.print(msg).unwrap();
                                }
                                Action::TempSend => { // Currently not working
                                    tokio::task::spawn(send_test_message(client.clone(), sender.clone())).await.unwrap().unwrap();
                                }
                            }
                        },
                        _ => break,
                    }
                }
            }
        }
    });
    Ok(())
}

async fn send_test_message(mut client: Client, sender: Sender<Action>) -> Result<()> {
    let plaintext = "\"Did you ever hear the tragedy of Darth Plagueis The Wise? I thought not. It’s not a story the Jedi would tell you. It’s a Sith legend. Darth Plagueis was a Dark Lord of the Sith, so powerful and so wise he could use the Force to influence the midichlorians to create life… He had such a knowledge of the dark side that he could even keep the ones he cared about from dying. The dark side of the Force is a pathway to many abilities some consider to be unnatural. He became so powerful… the only thing he was afraid of was losing his power, which eventually, of course, he did. Unfortunately, he taught his apprentice everything he knew, then his apprentice killed him in his sleep. Ironic. He could save others from death, but not himself.\" - Darth Sidious.";
    let b32_data = encrypt(plaintext.as_bytes(), b"bababooey")?; // Add a method to split up messages into 63 char labels.

    for name in b32_data {
        let domain_name = format!("{}", str::from_utf8(&name.as_bytes())?);
        sender.send(Action::Log(format!("Attempting to query with {}", domain_name))).await.ok();

        let response = client.query(
            domain_name.into_name()?,
            DNSClass::IN,
            RecordType::A,
        ).await?;
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        sender.send(Action::Log(format!("The server received and successfully echoed back this domain name: {}", response.answers()[0].name().to_string().to_uppercase()))).await.ok();

    }

    Ok(())
}