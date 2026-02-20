use std::env;
use crate::engine::ShellState;

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
