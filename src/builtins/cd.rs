use std::env;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO_CD: CommandInfo = CommandInfo {
    name: "cd",
    description: "Change the shell working directory.",
    usage: "cd [dir]\n\nChange the current directory to DIR. The default DIR is the value of the HOME shell variable.",
    run: cd_runner,
};

pub const COMMAND_INFO_PWD: CommandInfo = CommandInfo {
    name: "pwd",
    description: "Print the name of the current working directory.",
    usage: "pwd\n\nPrint the absolute pathname of the current working directory.",
    run: pwd_runner,
};

pub fn pwd_runner(_args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    pwd();
    (ExecutionResult::KeepRunning, 0)
}

pub fn cd_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    match run(args, state) {
        Ok(()) => (ExecutionResult::KeepRunning, 0),
        Err(e) => {
            eprintln!("cerf: cd: {}", e);
            (ExecutionResult::KeepRunning, 1)
        }
    }
}

pub fn run(args: &[String], state: &mut ShellState) -> Result<(), String> {
    let current = env::current_dir().map_err(|e| e.to_string())?;

    let target = if args.is_empty() {
        dirs::home_dir().ok_or("Could not find home directory".to_string())?
    } else if args[0] == "-" {
        state.previous_dir.clone().ok_or("OLDPWD not set".to_string())?
    } else {
        crate::engine::expand_home(&args[0])
    };

    if let Err(_) = env::set_current_dir(&target) {
        // Standard error message
        return Err(format!("no such file or directory: {}", target.display()));
    }
    
    state.previous_dir = Some(current);
    Ok(())
}

pub fn pwd() {
    match env::current_dir() {
        Ok(path) => println!("{}", path.display()),
        Err(e) => eprintln!("pwd: {}", e),
    }
}
