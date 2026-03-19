use crate::builtins::registry::CommandInfo;
use crate::engine::state::{ExecutionResult, ShellState};

pub const COMMAND_INFO_ULIMIT: CommandInfo = CommandInfo {
    name: "sys.ulimit",
    description: "Modify shell resource limits.",
    usage: "sys.ulimit [-SHabcdefiklmnpqrstuvxPT] [limit]\n\nModify shell resource limits. (Stub functionality)",
    run: ulimit_runner,
};

pub fn ulimit_runner(args: &[String], _state: &mut ShellState) -> (ExecutionResult, i32) {
    if !args.is_empty() {
        if args[0] == "-a" || args[0] == "-aH" || args[0] == "-aS" {
            println!("core file size          (blocks, -c) 0");
            println!("data seg size           (kbytes, -d) unlimited");
            println!("scheduling priority             (-e) 0");
            println!("file size               (blocks, -f) unlimited");
            println!("pending signals                 (-i) 0");
            println!("max locked memory       (kbytes, -l) unlimited");
            println!("max memory size         (kbytes, -m) unlimited");
            println!("open files                      (-n) 1024");
            println!("pipe size            (512 bytes, -p) 8");
            println!("POSIX message queues     (bytes, -q) 819200");
            println!("real-time priority              (-r) 0");
            println!("stack size              (kbytes, -s) 8192");
            println!("cpu time               (seconds, -t) unlimited");
            println!("max user processes              (-u) unlimited");
            println!("virtual memory          (kbytes, -v) unlimited");
            println!("file locks                      (-x) unlimited");
            return (ExecutionResult::KeepRunning, 0);
        }
    }

    println!("unlimited");
    (ExecutionResult::KeepRunning, 0)
}
