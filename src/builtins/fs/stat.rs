use std::fs;
use chrono::{DateTime, Local};
use crate::engine::state::{ExecutionResult, ShellState};
use crate::builtins::registry::CommandInfo;
use crate::engine::path::expand_home;

pub const COMMAND_INFO: CommandInfo = CommandInfo {
    name: "fs.stat",
    description: "Display file or file system status.",
    usage: "fs.stat [file ...]\n\nDisplay file or file system status.",
    run: runner,
};

pub fn runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if args.is_empty() {
        eprintln!("cerf: fs.stat: missing operand");
        return (ExecutionResult::KeepRunning, 1);
    }

    let mut exit_code = 0;
    for arg in args {
        let path = expand_home(arg);
        match fs::metadata(&path) {
            Ok(meta) => {
                println!("  File: {}", arg);
                let file_type = if meta.is_dir() {
                    "directory"
                } else if meta.is_file() {
                    "regular file"
                } else if meta.file_type().is_symlink() {
                    "symbolic link"
                } else {
                    "special file"
                };

                println!(
                    "  Size: {:<15} Blocks: {:<10} IO Block: {:<10} {}",
                    meta.len(),
                    (meta.len() + 511) / 512, // Rough estimation of 512-byte blocks
                    4096,                     // IO block size (typical)
                    file_type
                );

                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    let file_attributes = meta.file_attributes();
                    println!("Device: unknown         Inode: unknown         Links: unknown");
                    println!(
                        "Access: ({:o})  Uid: unknown   Gid: unknown",
                        file_attributes & 0o777
                    );
                }

                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    println!(
                        "Device: {:<15x} Inode: {:<15} Links: {}",
                        meta.dev(),
                        meta.ino(),
                        meta.nlink()
                    );
                    println!(
                        "Access: ({:04o})  Uid: ({:5})   Gid: ({:5})",
                        meta.mode() & 0o7777,
                        meta.uid(),
                        meta.gid()
                    );
                }

                #[cfg(not(any(windows, unix)))]
                {
                    println!("Device: unknown         Inode: unknown         Links: unknown");
                    println!("Access: unknown         Uid: unknown           Gid: unknown");
                }
                
                if let Ok(atime) = meta.accessed() {
                    let dt: DateTime<Local> = atime.into();
                    println!("Access: {}", dt.format("%Y-%m-%d %H:%M:%S.%f %z"));
                }
                if let Ok(mtime) = meta.modified() {
                    let dt: DateTime<Local> = mtime.into();
                    println!("Modify: {}", dt.format("%Y-%m-%d %H:%M:%S.%f %z"));
                }
                if let Ok(ctime) = meta.created() {
                    let dt: DateTime<Local> = ctime.into();
                    println!("Birth:  {}", dt.format("%Y-%m-%d %H:%M:%S.%f %z"));
                }
            }
            Err(e) => {
                eprintln!("cerf: fs.stat: cannot stat '{}': {}", arg, e);
                exit_code = 1;
            }
        }
    }

    (ExecutionResult::KeepRunning, exit_code)
}
