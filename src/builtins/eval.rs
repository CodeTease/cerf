use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::parser;
use crate::engine;

pub const COMMAND_INFO_EVAL: CommandInfo = CommandInfo {
    name: "sys.eval",
    description: "Construct command by concatenating arguments.",
    usage: "sys.eval [arg ...]\n\nConstructs a command by concatenating arguments together and then executes it.",
    run: eval_runner,
};

pub fn eval_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        return (ExecutionResult::KeepRunning, 0);
    }
    
    let command_str = args.join(" ");
    
    if let Some(entries) = parser::parse_pipeline(&command_str, &state.variables) {
        return engine::execute_list(entries, state);
    } else {
        (ExecutionResult::KeepRunning, 0)
    }
}
