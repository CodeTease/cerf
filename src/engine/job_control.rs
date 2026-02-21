use crate::engine::state::{ShellState, JobState, Job, ProcessInfo};

#[cfg(unix)]
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
#[cfg(unix)]
use nix::unistd::Pid;

/// Put the shell back in the foreground
#[cfg(unix)]
pub fn restore_terminal(state: &ShellState) {
    if let (Some(term), Some(shell_pgid)) = (state.shell_term, state.shell_pgid) {
        let _ = nix::unistd::tcsetpgrp(term, shell_pgid);
    }
}

#[cfg(windows)]
pub fn restore_terminal(_state: &ShellState) {}

/// Wait for a specific job. If it is in foreground, also give it the terminal.
#[cfg(unix)]
pub fn wait_for_job(job_id: usize, state: &mut ShellState, fg: bool) -> i32 {
    let mut last_code = 0;
    
    // Give terminal to job
    if fg {
        if let Some(job) = state.jobs.get(&job_id) {
            let pgid = Pid::from_raw(job.pgid as i32);
            if let Some(term) = state.shell_term {
                let _ = nix::unistd::tcsetpgrp(term, pgid);
            }
        } else {
            return 0; // Job not found
        }
    }

    loop {
        let job = match state.jobs.get_mut(&job_id) {
            Some(j) => j,
            None => break,
        };
        
        let pgid = job.pgid;
        
        if job.is_stopped() {
            if fg {
                println!("\n[{}] Stopped  {}", job.id, job.command);
            }
            break;
        }
        if job.is_done() {
            if let JobState::Done(c) = job.state() {
                last_code = c;
            }
            if fg {
                state.jobs.remove(&job_id);
            }
            break;
        }
        
        if !fg {
            // Done waiting since we just wanted to perform an update or we don't block
            break;
        }
        
        let wait_res = waitpid(Pid::from_raw(- (pgid as i32)), Some(WaitPidFlag::WUNTRACED));
        match wait_res {
            Ok(WaitStatus::Exited(pid, code)) => {
                if let Some(job) = state.jobs.get_mut(&job_id) {
                    for p in &mut job.processes {
                        if p.pid == pid.as_raw() as u32 {
                            p.state = JobState::Done(code);
                            last_code = code;
                        }
                    }
                }
            },
            Ok(WaitStatus::Signaled(pid, sig, _)) => {
                let code = 128 + sig as i32;
                if let Some(job) = state.jobs.get_mut(&job_id) {
                    for p in &mut job.processes {
                        if p.pid == pid.as_raw() as u32 {
                            p.state = JobState::Done(code);
                            last_code = code;
                        }
                    }
                }
                if fg {
                    println!("\n[{}] Terminated  {}", job_id, sig);
                }
            },
            Ok(WaitStatus::Stopped(pid, _sig)) => {
                if let Some(job) = state.jobs.get_mut(&job_id) {
                    for p in &mut job.processes {
                        if p.pid == pid.as_raw() as u32 {
                            p.state = JobState::Stopped;
                        }
                    }
                }
            },
            Ok(WaitStatus::Continued(pid)) => {
                if let Some(job) = state.jobs.get_mut(&job_id) {
                    for p in &mut job.processes {
                        if p.pid == pid.as_raw() as u32 {
                            p.state = JobState::Running;
                        }
                    }
                }
            },
            Err(nix::errno::Errno::ECHILD) => {
                // No more children? Mark all running as done (we lost track?)
                if let Some(job) = state.jobs.get_mut(&job_id) {
                    for p in &mut job.processes {
                        if p.state == JobState::Running {
                            p.state = JobState::Done(last_code);
                        }
                    }
                }
            },
            _ => {}
        }
    }

    if fg {
        restore_terminal(state);
    }
    
    last_code
}

#[cfg(windows)]
pub fn wait_for_job(_job_id: usize, _state: &mut ShellState, _fg: bool) -> i32 {
    0
}

/// Update statuses of all jobs in the background (WNOHANG)
#[cfg(unix)]
pub fn update_jobs(state: &mut ShellState) {
    loop {
        let wait_res = waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WNOHANG | WaitPidFlag::WUNTRACED | WaitPidFlag::WCONTINUED));
        match wait_res {
            Ok(WaitStatus::Exited(pid, code)) => {
                update_pid_state(state, pid.as_raw() as u32, JobState::Done(code));
            },
            Ok(WaitStatus::Signaled(pid, sig, _)) => {
                update_pid_state(state, pid.as_raw() as u32, JobState::Done(128 + sig as i32));
            },
            Ok(WaitStatus::Stopped(pid, _sig)) => {
                update_pid_state(state, pid.as_raw() as u32, JobState::Stopped);
            },
            Ok(WaitStatus::Continued(pid)) => {
                update_pid_state(state, pid.as_raw() as u32, JobState::Running);
            },
            _ => break,
        }
    }
    
    // Print and remove done jobs
    let mut to_remove = Vec::new();
    for (&id, job) in &mut state.jobs {
        if job.is_done() {
            if !job.reported_done {
                println!("[{}] Done  {}", id, job.command);
                job.reported_done = true;
            }
            to_remove.push(id);
        }
    }
    
    for id in to_remove {
        state.jobs.remove(&id);
    }
}

#[cfg(unix)]
fn update_pid_state(state: &mut ShellState, pid: u32, new_state: JobState) {
    for job in state.jobs.values_mut() {
        for p in &mut job.processes {
            if p.pid == pid {
                p.state = new_state.clone();
            }
        }
    }
}

#[cfg(windows)]
pub fn update_jobs(_state: &mut ShellState) {}

pub fn format_command(pipeline: &crate::parser::Pipeline) -> String {
    pipeline.commands.iter().map(|c| {
        let mut parts = vec![];
        if let Some(n) = &c.name {
            parts.push(n.clone());
        }
        parts.extend(c.args.iter().map(|a| a.value.clone()));
        parts.join(" ")
    }).collect::<Vec<_>>().join(" | ") + if pipeline.background { " &" } else { "" }
}
