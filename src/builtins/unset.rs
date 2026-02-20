use std::collections::HashMap;

/// Run the `unset` builtin.
///
/// Behaviour:
/// - `unset name …` → remove each named variable from shell and environment
pub fn run(args: &[String], variables: &mut HashMap<String, String>) {
    if args.is_empty() {
        return;
    }

    for arg in args {
        // Bash allows 'unset' to fail silently if the variable doesn't exist.
        // It also removes it from the environment.
        variables.remove(arg);
        unsafe { std::env::remove_var(arg); }
    }
}
