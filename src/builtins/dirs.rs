use std::env;
use std::io::Write;
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

// For now, we stub redirects internally as we change the signature to match BuiltinRunner
pub const COMMAND_INFO_PUSHD: CommandInfo = CommandInfo {
    name: "pushd",
    description: "Add a directory to the directory stack, or rotate the stack.",
    usage: "pushd [-n] [+N | -N | dir]\n\nAdds a directory to the top of the directory stack, or rotates the stack, making the new top of the stack the current working directory.",
    run: pushd_runner,
};

pub fn pushd_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    match pushd(args, state, None) {
        Ok(()) => (ExecutionResult::KeepRunning, 0),
        Err(e) => {
            eprintln!("cerf: {}", e);
            (ExecutionResult::KeepRunning, 1)
        }
    }
}

pub const COMMAND_INFO_POPD: CommandInfo = CommandInfo {
    name: "popd",
    description: "Remove directories from the directory stack.",
    usage: "popd [-n] [+N | -N]\n\nRemoves entries from the directory stack.",
    run: popd_runner,
};

pub fn popd_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    match popd(args, state, None) {
        Ok(()) => (ExecutionResult::KeepRunning, 0),
        Err(e) => {
            eprintln!("cerf: {}", e);
            (ExecutionResult::KeepRunning, 1)
        }
    }
}

pub const COMMAND_INFO_DIRS: CommandInfo = CommandInfo {
    name: "dirs",
    description: "Display the list of currently remembered directories.",
    usage: "dirs [-clpv] [+N] [-N]\n\nDisplay the list of currently remembered directories.",
    run: dirs_runner,
};

pub fn dirs_runner(_args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    run_dirs(state, None);
    (ExecutionResult::KeepRunning, 0)
}

pub fn pushd(args: &[String], state: &mut ShellState, stdout_redirect: Option<std::fs::File>) -> Result<(), String> {
    let current = env::current_dir().map_err(|e| e.to_string())?;

    if args.is_empty() {
        if state.dir_stack.is_empty() {
            return Err("pushd: no other directory".to_string());
        }

        let top = state.dir_stack.pop().unwrap();
        
        if let Err(_) = env::set_current_dir(&top) {
            state.dir_stack.push(top.clone());
            return Err(format!("pushd: no such file or directory: {}", top.display()));
        }

        state.previous_dir = Some(current.clone());
        state.dir_stack.push(current);
        
        run_dirs(state, stdout_redirect);
        return Ok(());
    }

    let target = crate::engine::expand_home(&args[0]);

    if let Err(_) = env::set_current_dir(&target) {
        return Err(format!("pushd: no such file or directory: {}", target.display()));
    }

    state.previous_dir = Some(current.clone());
    state.dir_stack.push(current);
    
    run_dirs(state, stdout_redirect);
    Ok(())
}

pub fn popd(_args: &[String], state: &mut ShellState, stdout_redirect: Option<std::fs::File>) -> Result<(), String> {
    if state.dir_stack.is_empty() {
        return Err("popd: directory stack empty".to_string());
    }

    let current = env::current_dir().map_err(|e| e.to_string())?;
    let target = state.dir_stack.pop().unwrap();

    if let Err(_) = env::set_current_dir(&target) {
        state.dir_stack.push(target.clone());
        return Err(format!("popd: no such file or directory: {}", target.display()));
    }

    state.previous_dir = Some(current);
    
    run_dirs(state, stdout_redirect);
    Ok(())
}

pub fn run_dirs(state: &ShellState, stdout_redirect: Option<std::fs::File>) {
    if let Ok(current) = env::current_dir() {
        if let Some(mut f) = stdout_redirect {
            let _ = write!(f, "{}", current.display());
            for dir in state.dir_stack.iter().rev() {
                let _ = write!(f, " {}", dir.display());
            }
            let _ = writeln!(f);
        } else {
            print!("{}", current.display());
            for dir in state.dir_stack.iter().rev() {
                print!(" {}", dir.display());
            }
            println!();
        }
    }
}
