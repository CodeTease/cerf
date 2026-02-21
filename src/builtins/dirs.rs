use std::env;
use std::io::Write;
use crate::engine::ShellState;

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
