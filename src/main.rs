mod parser;
mod engine;
mod builtins;
mod signals;

use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::env;
use engine::ShellState;

fn get_prompt() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let home = dirs::home_dir();
    
    let path_str = if let Some(home) = home {
        if cwd.starts_with(&home) {
            let relative = cwd.strip_prefix(&home).unwrap();
            if relative.as_os_str().is_empty() {
                "~".to_string()
            } else {
                format!("~/{}", relative.display())
            }
        } else {
            cwd.display().to_string()
        }
    } else {
        cwd.display().to_string()
    };
    
    format!("cf {} > ", path_str)
}

fn main() -> rustyline::Result<()> {
    signals::init();
    let mut rl = DefaultEditor::new()?;
    let mut state = ShellState::new();

    loop {
        let prompt = get_prompt();
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                let input = line.trim();
                if input.is_empty() {
                    continue;
                }
                let _ = rl.add_history_entry(input);

                if let Some(entries) = parser::parse_pipeline(input) {
                    match engine::execute_list(entries, &mut state) {
                        engine::ExecutionResult::Exit => break,
                        engine::ExecutionResult::KeepRunning => {},
                    }
                }
            },
            Err(ReadlineError::Interrupted) => {
                continue;
            },
            Err(ReadlineError::Eof) => {
                println!("exit");
                break;
            },
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }
    Ok(())
}
