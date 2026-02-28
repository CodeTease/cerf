use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "job.list",
    description: "Display status of jobs.",
    usage: "job.list\n\nLists the active jobs. JOBSpec restricts output to that job.",
    run: jobs_runner,
};

pub fn jobs_runner(_args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    let code = run(state);
    (ExecutionResult::KeepRunning, code)
}

pub fn run(state: &ShellState) -> i32 {
    let mut jobs: Vec<_> = state.jobs.iter().collect();
    jobs.sort_by_key(|&(&id, _)| id);
    for (&id, job) in jobs {
        let status_str = match job.state() {
            crate::engine::JobState::Running => "Running",
            crate::engine::JobState::Stopped => "Stopped",
            crate::engine::JobState::Done(_) => "Done",
        };
        println!("[{}] {}  {}", id, status_str, job.command);
    }
    0
}
