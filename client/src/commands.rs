use std::collections::HashMap;
use shared::{Action, Command, CommandMap};
use shared::State;

fn exit(state: &mut State, _args: &[&str]) -> color_eyre::Result<()> {
    state.printer.print("Exit called!".into())?;
    state.exit = true;

    Ok(())
}

fn help(state: &mut State, _args: &[&str]) -> color_eyre::Result<()> {
    println!("\nCommands\n========"); // commands to implement, clients (show list of clients), info (show server info), connect (init message session with a client), etc.
    let max_len = state.commands.keys()
        .map(|cmd_name| cmd_name.len())
        .reduce(|name_len, new_len| name_len.max(new_len))
        .unwrap_or(0);


    for (cmd_str, cmd) in &state.commands {
        state.printer.print(format!("{:max_len$}  -\t{}", cmd_str, cmd.description).into())?;
    }

    Ok(())
}

fn test(state: &mut State, _args: &[&str]) -> color_eyre::Result<()> {
    println!("\nSending test message...");
    state.sender.blocking_send(Action::TempSend).ok();

    Ok(())
}

pub fn init_commands() -> CommandMap {
    let mut commands: CommandMap = HashMap::new();
    commands.insert("exit", Command::new("Stops the server", exit, false, false, None));
    commands.insert("help", Command::new("Shows this menu", help, false, false, None));
    commands.insert("test", Command::new("TEMP - Sends a test message", test, false, false, None));
    commands
}

// Just make it so that there's a generic messaging mode that lets received messages be shown