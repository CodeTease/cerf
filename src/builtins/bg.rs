use crate::engine::ShellState;

pub fn run(args: &[String], state: &mut ShellState) -> i32 {
    let mut job_id = None;
    if args.is_empty() {
        if let Some((&id, _)) = state.jobs.iter().max_by_key(|(&id, _)| *id) {
            job_id = Some(id);
        }
    } else {
        if let Some(id_str) = args[0].strip_prefix('%') {
            job_id = id_str.parse().ok();
        } else {
            job_id = args[0].parse().ok();
        }
    }

    if let Some(id) = job_id {
        if let Some(job) = state.jobs.get_mut(&id) {
            println!("[{}] {}", id, job.command);
            job.reported_done = false;
            for p in &mut job.processes {
                if p.state == crate::engine::state::JobState::Stopped {
                    p.state = crate::engine::state::JobState::Running;
                }
            }
            
            #[cfg(unix)]
            {
                let pgid = job.pgid;
                let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(-(pgid as i32)), nix::sys::signal::Signal::SIGCONT);
            }
            
            0
        } else {
            eprintln!("cerf: bg: %{}: no such job", id);
            1
        }
    } else {
        eprintln!("cerf: bg: current: no such job");
        1
    }
}
