use crate::engine::state::{ShellState, JobState};

#[cfg(unix)]
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
#[cfg(unix)]
use nix::unistd::Pid;

/// Put the shell back in the foreground
#[cfg(unix)]
pub fn restore_terminal(state: &ShellState) {
    if let (Some(term), Some(shell_pgid)) = (state.shell_term, state.shell_pgid) {
        let _ = nix::unistd::tcsetpgrp(unsafe { std::os::fd::BorrowedFd::borrow_raw(term) }, shell_pgid);
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
                let _ = nix::unistd::tcsetpgrp(unsafe { std::os::fd::BorrowedFd::borrow_raw(term) }, pgid);
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
        
        let wait_res = waitpid(Pid::from_raw(-1), Some(WaitPidFlag::WUNTRACED));
        match wait_res {
            Ok(WaitStatus::Exited(pid, code)) => {
                update_pid_state(state, pid.as_raw() as u32, JobState::Done(code));
            },
            Ok(WaitStatus::Signaled(pid, sig, _)) => {
                let code = 128 + sig as i32;
                update_pid_state(state, pid.as_raw() as u32, JobState::Done(code));
                if fg {
                    if let Some(job) = state.jobs.get(&job_id) {
                        if job.processes.iter().any(|p| p.pid == pid.as_raw() as u32) {
                            println!("\n[{}] Terminated  {}", job_id, sig);
                        }
                    }
                }
            },
            Ok(WaitStatus::Stopped(pid, _sig)) => {
                update_pid_state(state, pid.as_raw() as u32, JobState::Stopped);
            },
            Ok(WaitStatus::Continued(pid)) => {
                update_pid_state(state, pid.as_raw() as u32, JobState::Running);
            },
            Err(nix::errno::Errno::ECHILD) => {
                // No more children at all
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
pub fn wait_for_job(job_id: usize, state: &mut ShellState, fg: bool) -> i32 {

    let mut last_code = 0;

    if fg {
        crate::FG_JOB.store(job_id, std::sync::atomic::Ordering::Relaxed);
    }

    loop {
        let job = match state.jobs.get_mut(&job_id) {
            Some(j) => j,
            None => break,
        };
        
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
            break;
        }

        // Pump messages from the generic IOCP receiver if available
        crate::engine::job_control::pump_iocp(state);
        
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    
    if fg {
        crate::FG_JOB.store(0, std::sync::atomic::Ordering::Relaxed);
    }
    
    last_code
}

#[cfg(windows)]
pub fn pump_iocp(state: &mut ShellState) {
    let mut msgs = Vec::new();
    if let Some(rx) = &state.iocp_receiver {
        while let Ok(msg) = rx.try_recv() {
            msgs.push(msg);
        }
    }
    for msg in msgs {
        handle_iocp_msg(state, msg);
    }
}

#[cfg(windows)]
pub fn handle_iocp_msg(state: &mut ShellState, msg: IocpMessage) {
    use windows_sys::Win32::System::SystemServices::{JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO, JOB_OBJECT_MSG_EXIT_PROCESS, JOB_OBJECT_MSG_ABNORMAL_EXIT_PROCESS};
    let event_job_id = msg.job_id;
    let pid = msg.pid;

    if msg.msg == JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO {
        if let Some(j) = state.jobs.get_mut(&event_job_id) {
            for p in &mut j.processes {
                if p.state == JobState::Running {
                    p.state = JobState::Done(0);
                }
            }
        }
    } else if msg.msg == JOB_OBJECT_MSG_EXIT_PROCESS || msg.msg == JOB_OBJECT_MSG_ABNORMAL_EXIT_PROCESS {
        let mut exit_code = 0;
        unsafe {
            let proc_handle = windows_sys::Win32::System::Threading::OpenProcess(
                windows_sys::Win32::System::Threading::PROCESS_QUERY_LIMITED_INFORMATION,
                0,
                pid
            );
            if !proc_handle.is_null() {
                windows_sys::Win32::System::Threading::GetExitCodeProcess(proc_handle, &mut exit_code);
                windows_sys::Win32::Foundation::CloseHandle(proc_handle);
            } else if msg.msg == JOB_OBJECT_MSG_ABNORMAL_EXIT_PROCESS {
                exit_code = 1;
            }
        }
        update_pid_state(state, pid, JobState::Done(exit_code as i32));
    }
    
    // Check if job is fully done and report if it was a background job
    // Actually handled by update_jobs printing
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
pub fn update_jobs(state: &mut ShellState) {
    pump_iocp(state);
    
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

pub fn set_current_job(state: &mut ShellState, job_id: usize) {
    if state.current_job == Some(job_id) {
        return;
    }
    state.previous_job = state.current_job;
    state.current_job = Some(job_id);
}

pub fn resolve_job_specifier(arg: &str, state: &ShellState) -> Result<usize, String> {
    if arg == "%+" || arg == "%%" {
        return state.current_job.ok_or_else(|| "current: no such job".to_string());
    } else if arg == "%-" {
        return state.previous_job.ok_or_else(|| "previous: no such job".to_string());
    } else if let Some(id_str) = arg.strip_prefix('%') {
        if let Ok(id) = id_str.parse::<usize>() {
            return Ok(id);
        } else {
            // Find by string prefix
            let mut matches = Vec::new();
            for (&id, job) in &state.jobs {
                if job.command.starts_with(id_str) {
                    matches.push(id);
                }
            }
            if matches.is_empty() {
                return Err(format!("%{}: no such job", id_str));
            } else if matches.len() > 1 {
                return Err(format!("%{}: ambiguous job specifier", id_str));
            } else {
                return Ok(matches[0]);
            }
        }
    } else if let Ok(id) = arg.parse::<usize>() {
        return Ok(id);
    }
    Err(format!("{}: invalid job specifier", arg))
}

#[cfg(windows)]
pub struct IocpMessage {
    pub msg: u32,
    pub job_id: usize,
    pub pid: u32,
}
