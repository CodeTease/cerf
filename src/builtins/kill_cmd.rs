use crate::engine::ShellState;

pub fn run(args: &[String], state: &mut ShellState) -> i32 {
    if args.is_empty() {
        eprintln!("cerf: kill: usage: kill [-s sigspec] pid | jobspec ...");
        return 1;
    }
    
    let mut targets = Vec::new();
    #[cfg(unix)]
    let mut sig = nix::sys::signal::Signal::SIGTERM;
    
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
            i += 2;
            continue;
        } else if args[i].starts_with('-') && args[i].len() > 1 {
            #[cfg(unix)]
            {
                let s = &args[i][1..];
                if s == "9" { sig = nix::sys::signal::Signal::SIGKILL; }
                else if s == "KILL" { sig = nix::sys::signal::Signal::SIGKILL; }
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
        eprintln!("cerf: kill: not fully supported on windows");
        code = 1;
    }
    
    code
}
