use crate::engine::ShellState;

pub fn run_tether(args: &[String], state: &mut ShellState) -> i32 {
    set_tether(args, state, true)
}

pub fn run_untether(args: &[String], state: &mut ShellState) -> i32 {
    set_tether(args, state, false)
}

fn set_tether(args: &[String], state: &mut ShellState, tether: bool) -> i32 {
    let mut code = 0;
    
    if args.is_empty() {
        if tether {
            eprintln!("cerf: tether: usage: tether jobspec ...");
        } else {
            eprintln!("cerf: untether: usage: untether jobspec ...");
        }
        return 1;
    }

    for arg in args {
        match crate::engine::job_control::resolve_job_specifier(arg, state) {
            Ok(id) => {
                if let Some(job) = state.jobs.get(&id) {
                    #[cfg(windows)]
                    {
                        unsafe {
                            let mut limit_info: windows_sys::Win32::System::JobObjects::JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                            if tether {
                                limit_info.BasicLimitInformation.LimitFlags = windows_sys::Win32::System::JobObjects::JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                            }
                            let success = windows_sys::Win32::System::JobObjects::SetInformationJobObject(
                                job.job_handle as _,
                                windows_sys::Win32::System::JobObjects::JobObjectExtendedLimitInformation,
                                &limit_info as *const _ as *const std::ffi::c_void,
                                std::mem::size_of_val(&limit_info) as u32,
                            );
                            if success == 0 {
                                eprintln!("cerf: failed to set tether on job {}", id);
                                code = 1;
                            } else {
                                if tether {
                                    println!("[{}] tethered", id);
                                } else {
                                    println!("[{}] untethered", id);
                                }
                            }
                        }
                    }
                    #[cfg(unix)]
                    {
                        eprintln!("cerf: tether/untether is not supported on Unix.");
                        code = 1;
                    }
                }
            }
            Err(e) => {
                eprintln!("cerf: {}", e);
                code = 1;
            }
        }
    }
    
    code
}
