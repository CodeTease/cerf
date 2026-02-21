use crate::engine::ShellState;

pub fn run(state: &ShellState) -> i32 {
    let mut jobs: Vec<_> = state.jobs.iter().collect();
    jobs.sort_by_key(|&(&id, _)| id);
    for (&id, job) in jobs {
        println!("[{}] {}  {}", id, job.state(), job.command);
    }
    0
}
