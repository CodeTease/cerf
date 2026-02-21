pub fn exit() {
    // No-op here, handled by engine return value
}

pub fn clear() {
    use std::io::{self, Write};
    print!("\x1B[2J\x1B[1;1H");
    let _ = io::stdout().flush();
}
