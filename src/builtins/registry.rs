use crate::engine::{ExecutionResult, ShellState};
use crate::builtins;


pub type BuiltinRunner = fn(&[String], &mut ShellState) -> (ExecutionResult, i32);

pub struct CommandInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub usage: &'static str,
    pub run: BuiltinRunner,
}

pub const BUILTINS: &[CommandInfo] = &[
    builtins::alias::COMMAND_INFO,
    builtins::bg::COMMAND_INFO,
    builtins::boolean::COMMAND_INFO_FALSE,
    builtins::boolean::COMMAND_INFO_TRUE,
    builtins::cd::COMMAND_INFO_CD,
    builtins::cd::COMMAND_INFO_PWD,
    builtins::dirs::COMMAND_INFO_DIRS,
    builtins::dirs::COMMAND_INFO_POPD,
    builtins::dirs::COMMAND_INFO_PUSHD,
    builtins::echo::COMMAND_INFO,
    builtins::export::COMMAND_INFO,
    builtins::fg::COMMAND_INFO,
    builtins::help::COMMAND_INFO,
    builtins::history::COMMAND_INFO,
    builtins::jobs::COMMAND_INFO,
    builtins::kill_cmd::COMMAND_INFO,
    builtins::read::COMMAND_INFO,
    builtins::set::COMMAND_INFO,
    builtins::source::COMMAND_INFO_DOT,
    builtins::source::COMMAND_INFO_SOURCE,
    builtins::system::COMMAND_INFO_CLEAR,
    builtins::system::COMMAND_INFO_EXEC,
    builtins::system::COMMAND_INFO_EXIT,
    builtins::test_cmd::COMMAND_INFO_BRACKET,
    builtins::test_cmd::COMMAND_INFO_TEST,
    builtins::tether::COMMAND_INFO_TETHER,
    builtins::tether::COMMAND_INFO_UNTETHER,
    builtins::type_cmd::COMMAND_INFO,
    builtins::unalias::COMMAND_INFO,
    builtins::unset::COMMAND_INFO,
    builtins::wait::COMMAND_INFO,
];

pub fn find_command(name: &str) -> Option<&'static CommandInfo> {
    BUILTINS.iter().find(|cmd| cmd.name == name)
}
