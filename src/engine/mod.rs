mod alias;
mod execution;
mod glob;
pub mod job_control;
pub mod path;
mod redirect;
pub mod state;

// Re-export the public API so that external code (`main.rs`, `builtins/`)
// can continue to use `engine::ShellState`, `engine::ExecutionResult`, etc.
pub use execution::execute_list;
pub use path::{expand_home, find_executable};
pub use state::{ExecutionResult, JobState, ShellState};
