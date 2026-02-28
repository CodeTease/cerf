use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "job.bg",
    description: "Move jobs to the background.",
    usage: "job.bg [job_spec ...]\n\nPlace the jobs identified by each JOB_SPEC in the background, as if they had been started with `&`.",
    run: bg_runner,
};

pub fn bg_runner(args: &[String], state: &mut ShellState) -> (ExecutionResult, i32) {
    let code = run(args, state);
    (ExecutionResult::KeepRunning, code)
}

pub fn run(args: &[String], state: &mut ShellState) -> i32 {
    let mut job_id = None;
    if args.is_empty() {
        if let Ok(id) = crate::engine::job_control::resolve_job_specifier("%+", state) {
            job_id = Some(id);
        } else if let Some(&id) = state.jobs.keys().max() {
            job_id = Some(id);
        }
    } else {
        if let Ok(id) = crate::engine::job_control::resolve_job_specifier(&args[0], state) {
            job_id = Some(id);
        } else {
            eprintln!("cerf: bg: {}", crate::engine::job_control::resolve_job_specifier(&args[0], state).unwrap_err());
            return 1;
        }
    }

    if let Some(id) = job_id {
        if let Some(job) = state.jobs.get_mut(&id) {
            println!("[{}] {}", id, job.command);
            job.reported_done = false;
            for p in &mut job.processes {
                if p.state == crate::engine::JobState::Stopped {
                    p.state = crate::engine::JobState::Running;
                }
            }
            
            #[cfg(unix)]
            {
                let pgid = job.pgid;
                let _ = nix::sys::signal::kill(nix::unistd::Pid::from_raw(-(pgid as i32)), nix::sys::signal::Signal::SIGCONT);
            }
            
            #[cfg(windows)]
            {
                let pids = crate::builtins::kill_cmd::get_job_pids(job.job_handle);
                for pid in pids {
                    crate::builtins::kill_cmd::suspend_or_resume_process_win(pid, false);
                }
            }
            
            crate::engine::job_control::set_current_job(state, id);
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
