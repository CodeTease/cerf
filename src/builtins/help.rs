// PLACEHOLDER
// TODO: Actually implement help

pub fn run(args: &[String]) -> String {
    if args.is_empty() {
        let mut help_text = String::new();
        help_text.push_str("cerf, version 0.1.0\n");
        help_text.push_str("These shell commands are defined internally.  Type `help' to see this list.\n");
        help_text.push_str("Type `help name' to find out more about the function `name'.\n");
        help_text.push_str("Use `man -k' or `info' to find out more about commands not in this list.\n\n");
        help_text.push_str(" alias [name[=value] ... ]\n");
        help_text.push_str(" bg [job_spec ...]\n");
        help_text.push_str(" cd [dir]\n");
        help_text.push_str(" clear\n");
        help_text.push_str(" dirs [-clpv]\n");
        help_text.push_str(" echo [arg ...]\n");
        help_text.push_str(" exec [command [arguments ...]]\n");
        help_text.push_str(" exit [n]\n");
        help_text.push_str(" export [name[=value] ...]\n");
        help_text.push_str(" false\n");
        help_text.push_str(" fg [job_spec]\n");
        help_text.push_str(" help [pattern ...]\n");
        help_text.push_str(" history\n");
        help_text.push_str(" jobs\n");
        help_text.push_str(" kill [-s sigspec | -n signum | -sigspec] pid | jobspec ... or kill -l\n");
        help_text.push_str(" popd [-n] [+N | -N]\n");
        help_text.push_str(" pushd [-n] [+N | -N | dir]\n");
        help_text.push_str(" pwd\n");
        help_text.push_str(" read [arg ...]\n");
        help_text.push_str(" set\n");
        help_text.push_str(" source filename [arguments]\n");
        help_text.push_str(" test [expr]\n");
        help_text.push_str(" tether [pid]\n");
        help_text.push_str(" true\n");
        help_text.push_str(" type [-afptP] name [name ...]\n");
        help_text.push_str(" unalias [-a] name [name ...]\n");
        help_text.push_str(" unset [name ...]\n");
        help_text.push_str(" untether [pid]\n");
        help_text.push_str(" wait [id]\n");
        help_text.push_str(" [ arg... ]\n");
        help_text
    } else {
        format!("cerf: help: no help topics match `{}`\n", args[0])
    }
}
