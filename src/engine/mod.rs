mod state;
mod redirect;
mod alias;
pub mod path;
mod execution;

// Re-export the public API so that external code (`main.rs`, `builtins/`)
// can continue to use `engine::ShellState`, `engine::ExecutionResult`, etc.
pub use state::{ShellState, ExecutionResult};
pub use execution::execute_list;
pub use path::{expand_home, find_executable};
