use crate::engine::ShellState;

pub fn run(args: &[String], state: &mut ShellState) -> i32 {
    if args.is_empty() {
        eprintln!("cerf: kill: usage: kill [-s sigspec] pid | jobspec ...");
        return 1;
    }
    
    let mut targets = Vec::new();
    #[cfg(unix)]
    let mut sig = nix::sys::signal::Signal::SIGTERM;
    #[cfg(windows)]
    let mut sig = 15; // SIGTERM
    
    let mut i = 0;
    while i < args.len() {
        if args[i] == "-s" && i + 1 < args.len() {
            #[cfg(unix)]
            {
                if args[i+1] == "KILL" || args[i+1] == "9" {
                    sig = nix::sys::signal::Signal::SIGKILL;
                } else if args[i+1] == "STOP" {
                    sig = nix::sys::signal::Signal::SIGSTOP;
                } else if args[i+1] == "CONT" {
                    sig = nix::sys::signal::Signal::SIGCONT;
                } else if args[i+1] == "INT" {
                    sig = nix::sys::signal::Signal::SIGINT;
                }
            }
            #[cfg(windows)]
            {
                if args[i+1] == "KILL" || args[i+1] == "9" {
                    sig = 9;
                } else if args[i+1] == "STOP" {
                    sig = 19; // SIGSTOP
                } else if args[i+1] == "CONT" {
                    sig = 18; // SIGCONT
                } else if args[i+1] == "INT" {
                    sig = 2; // SIGINT
                }
            }
            i += 2;
            continue;
        } else if args[i].starts_with('-') && args[i].len() > 1 {
            #[cfg(unix)]
            {
                let s = &args[i][1..];
                if s == "9" { sig = nix::sys::signal::Signal::SIGKILL; }
                else if s == "KILL" { sig = nix::sys::signal::Signal::SIGKILL; }
            }
            #[cfg(windows)]
            {
                let s = &args[i][1..];
                if s == "9" { sig = 9; }
                else if s == "KILL" { sig = 9; }
            }
            i += 1;
            continue;
        }
        
        targets.push(&args[i]);
        i += 1;
    }
    
    let mut code = 0;
    #[cfg(unix)]
    {
        for target in targets {
            let mut pids_to_kill = Vec::new();
            
            if let Some(id_str) = target.strip_prefix('%') {
                if let Ok(id) = id_str.parse::<usize>() {
                    if let Some(job) = state.jobs.get(&id) {
                        pids_to_kill.push(-(job.pgid as i32));
                    } else {
                        eprintln!("cerf: kill: %{}: no such job", id);
                        code = 1;
                        continue;
                    }
                }
            } else if let Ok(pid) = target.parse::<i32>() {
                pids_to_kill.push(pid);
            } else {
                eprintln!("cerf: kill: {}: arguments must be process or job IDs", target);
                code = 1;
                continue;
            }
            
            for pid in pids_to_kill {
                if let Err(e) = nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid), sig) {
                    eprintln!("cerf: kill: ({}) - {}", pid, e);
                    code = 1;
                }
            }
        }
    }
    
    #[cfg(windows)]
    {
        for target in targets {
            let mut job_handle_to_kill = None;
            let mut pids_to_kill = Vec::new();
            
            if let Some(id_str) = target.strip_prefix('%') {
                if let Ok(id) = id_str.parse::<usize>() {
                    if let Some(job) = state.jobs.get(&id) {
                        job_handle_to_kill = Some(job.job_handle);
                        pids_to_kill.extend(job.processes.iter().map(|p| p.pid));
                    } else {
                        eprintln!("cerf: kill: %{}: no such job", id);
                        code = 1;
                        continue;
                    }
                }
            } else if let Ok(pid) = target.parse::<u32>() {
                pids_to_kill.push(pid);
            } else {
                eprintln!("cerf: kill: {}: arguments must be process or job IDs", target);
                code = 1;
                continue;
            }
            
            if sig == 9 || sig == 15 || sig == 2 {
                if let Some(jh) = job_handle_to_kill {
                    unsafe {
                        windows_sys::Win32::System::JobObjects::TerminateJobObject(jh as _, 1);
                    }
                } else {
                    for pid in pids_to_kill {
                        unsafe {
                            let handle = windows_sys::Win32::System::Threading::OpenProcess(
                                windows_sys::Win32::System::Threading::PROCESS_TERMINATE,
                                0,
                                pid
                            );
                            if !handle.is_null() {
                                windows_sys::Win32::System::Threading::TerminateProcess(handle, 1);
                                windows_sys::Win32::Foundation::CloseHandle(handle);
                            } else {
                                code = 1;
                            }
                        }
                    }
                }
            } else if sig == 18 || sig == 19 {
                let suspend = sig == 19;
                for pid in pids_to_kill {
                    crate::builtins::kill_cmd::suspend_or_resume_process_win(pid, suspend);
                }
            }
        }
    }
    
    code
}

#[cfg(windows)]
pub fn suspend_or_resume_process_win(pid: u32, suspend: bool) {
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Thread32First, Thread32Next, THREADENTRY32, TH32CS_SNAPTHREAD
    };
    use windows_sys::Win32::System::Threading::{OpenThread, SuspendThread, ResumeThread, THREAD_SUSPEND_RESUME};
    use windows_sys::Win32::Foundation::CloseHandle;

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
        if snapshot != windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
            let mut te32: THREADENTRY32 = std::mem::zeroed();
            te32.dwSize = std::mem::size_of::<THREADENTRY32>() as u32;
            
            if Thread32First(snapshot, &mut te32) != 0 {
                loop {
                    if te32.th32OwnerProcessID == pid {
                        let thread_handle = OpenThread(THREAD_SUSPEND_RESUME, 0, te32.th32ThreadID);
                        if !thread_handle.is_null() {
                            if suspend {
                                SuspendThread(thread_handle);
                            } else {
                                ResumeThread(thread_handle);
                            }
                            CloseHandle(thread_handle);
                        }
                    }
                    if Thread32Next(snapshot, &mut te32) == 0 {
                        break;
                    }
                }
            }
            CloseHandle(snapshot);
        }
    }
}
