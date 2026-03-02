use crate::engine::state::{ExecutionResult, ShellState, Variable, VarValue};
use crate::builtins::registry::CommandInfo;
use std::collections::HashMap;

pub const COMMAND_INFO_DECLARE: CommandInfo = CommandInfo {
    name: "env.declare",
    description: "Declare variables and give them attributes.",
    usage: "env.declare [-aAirxi] [name[=value] ...]\n\nDeclare variables and give them attributes.",
    run: declare_runner,
};

pub fn declare_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    let mode = run(args, state, false);
    (ExecutionResult::KeepRunning, mode)
}

pub fn run(args: &[String], state: &mut ShellState, local_scope: bool) -> i32 {
    let mut i = 0;
    
    let mut make_array = false;
    let mut make_integer = false;
    let mut make_readonly = false;
    let mut make_export = false;

    while i < args.len() {
        let arg = &args[i];
        if arg.starts_with('-') && arg.len() > 1 {
            for ch in arg[1..].chars() {
                match ch {
                    'a' => make_array = true,
                    'i' => make_integer = true,
                    'r' => make_readonly = true,
                    'x' => make_export = true,
                    _ => {}
                }
            }
            i += 1;
        } else {
            break;
        }
    }

    let targets = &args[i..];
    
    if targets.is_empty() {
        // Just print
        return 0;
    }

    for target in targets {
        let mut name = target.as_str();
        let mut value_str = None;

        if let Some(eq_pos) = target.find('=') {
            name = &target[..eq_pos];
            value_str = Some(&target[eq_pos + 1..]);
        }
        
        // If not creating new, inherit old
        let mut old_var = state.get_var(name).cloned();
        
        let mut var = old_var.unwrap_or_else(|| Variable::new_string(String::new()));
        
        if make_array { var.value = VarValue::Array(Vec::new()); }
        if make_integer { var.integer = true; }
        if make_readonly { var.readonly = true; }
        if make_export { var.exported = true; }

        if let Some(val) = value_str {
            if var.readonly {
                eprintln!("cerf: declare: {}: readonly variable", name);
                return 1;
            }
            if make_array || matches!(var.value, VarValue::Array(_)) {
                // very simple array parsing: (1 2 3)
                let val_trim = val.trim();
                if val_trim.starts_with('(') && val_trim.ends_with(')') {
                    let inner = &val_trim[1..val_trim.len()-1];
                    let arr_vals: Vec<String> = inner.split_whitespace().map(|s| s.to_string()).collect();
                    var.value = VarValue::Array(arr_vals);
                } else {
                    var.value = VarValue::Array(vec![val.to_string()]);
                }
            } else {
                var.value = VarValue::String(val.to_string());
            }
        }
        
        if var.exported {
            unsafe { std::env::set_var(name, var.value.as_string()); }
        }
        
        if local_scope {
            state.set_local_var(name, var);
        } else {
            state.set_var(name, var);
        }
    }

    0
}
