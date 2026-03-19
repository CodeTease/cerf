use crate::builtins::registry::CommandInfo;
use crate::engine::state::{ExecutionResult, ShellState};

pub const COMMAND_INFO_SHIFT: CommandInfo = CommandInfo {
    name: "env.shift",
    description: "Shift positional parameters.",
    usage: "env.shift [n]\n\nRename the positional parameters $N+1,$N+2 ... to $1,$2 ...",
    run: shift_runner,
};

pub fn shift_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    let n = if args.is_empty() {
        1
    } else {
        match args[0].parse::<usize>() {
            Ok(num) => num,
            Err(_) => {
                eprintln!("cerf: shift: {}: numeric argument required", args[0]);
                return (ExecutionResult::KeepRunning, 1);
            }
        }
    };

    if n > state.positional_args.len() {
        return (ExecutionResult::KeepRunning, 1);
    }

    state.positional_args.drain(0..n);

    let mut idx = 1;
    loop {
        let key = idx.to_string();
        if state.variables.remove(&key).is_none() {
            break;
        }
        idx += 1;
    }

    let params = state.positional_args.clone();
    for (i, val) in params.iter().enumerate() {
        state.set_var(
            &(i + 1).to_string(),
            crate::engine::state::Variable::new_string(val.clone()),
        );
    }
    state.set_var(
        "#",
        crate::engine::state::Variable::new_string(params.len().to_string()),
    );

    (ExecutionResult::KeepRunning, 0)
}
