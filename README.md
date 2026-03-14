# Cerf Shell

**Cerf** is a modern, cross-platform shell written in Rust, providing a robust POSIX-like environment with advanced features like job control and structured control flow.

A **CodeTease** project. (CerfSh branch)

## Features

- **POSIX-ready:** Pipelines, redirections, and logic operators.
- **Control Flow:** `if`, `while`, `for`, `loop`, and function support.
- **Cross-platform:** Native support for Windows and Unix-like systems.
- **Interactive:** Command history, tab completion, and rich prompts.
- **Job Control:** Manage background tasks with `bg`, `fg`, and `jobs`.

## Quick Start

Run the executable to enter the interactive shell:

```cerf
cerf
```

### Examples

```cerf
# Multi-stage pipelines
cat file.txt | grep "rust" | wc -l

# Structured loops (Brace-style)
for file in src/*.rs { 
    echo "Checking $file" 
}

# Control flow
if test -f Cargo.toml { 
    echo "Rust project detected" 
} else { 
    echo "No Cargo.toml found" 
}

# Shell functions
func hello {
    echo "Hello, Cerf!"
}
hello
```

## Installation

<!-- Temporary leave nothing for the Installation section -->

## License

Licensed under the **Apache License 2.0**.
