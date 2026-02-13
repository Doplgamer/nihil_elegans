use std::collections::HashMap;
use shared::{Command, CommandMap, State};

fn exit(state: &mut State, _args: &[&str]) -> color_eyre::Result<()> {
    println!("Exit called!");
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

pub fn init_commands() -> CommandMap {
    let mut commands: CommandMap = HashMap::new();
    commands.insert("exit", Command::new("Stops the server", exit,
                                         // false, false, None
    ));
    commands.insert("help", Command::new("Shows this menu", help,
                                         // false, false, None
    ));

    commands
}