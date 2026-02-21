use crate::engine::ShellState;
use crate::engine::job_control::wait_for_job;

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
        if state.jobs.contains_key(&id) {
            println!("{}", state.jobs[&id].command);
            #[cfg(unix)]
            {
                let pgid = state.jobs[&id].pgid;
                let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(-(pgid as i32)), nix::sys::signal::Signal::SIGCONT);
                return wait_for_job(id, state, true);
            }
            #[cfg(windows)]
            {
                0
            }
        } else {
            eprintln!("cerf: fg: %{}: no such job", id);
            1
        }
    } else {
        eprintln!("cerf: fg: current: no such job");
        1
    }
}
