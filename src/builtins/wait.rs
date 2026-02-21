use crate::engine::ShellState;
use crate::engine::job_control::wait_for_job;

pub fn run(args: &[String], state: &mut ShellState) -> i32 {
    if args.is_empty() {
        let job_ids: Vec<_> = state.jobs.keys().cloned().collect();
        for id in job_ids {
            wait_for_job(id, state, false);
        }
        0
    } else {
        let job_id = if let Some(id_str) = args[0].strip_prefix('%') {
            id_str.parse().ok()
        } else {
            args[0].parse().ok()
        };
        
        if let Some(id) = job_id {
            if state.jobs.contains_key(&id) {
                wait_for_job(id, state, false)
            } else {
                eprintln!("cerf: wait: %{}: no such job", id);
                127
            }
        } else {
            eprintln!("cerf: wait: '{}': not a pid or valid job spec", args[0]);
            1
        }
    }
}
