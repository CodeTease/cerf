# Cerf

**Cerf** is a cross-platform shell written in Rust, providing a POSIX-like environment with core features like job control and structured control flow.

A **CodeTease** project. (CerfSh branch)

## Features

- **POSIX-inspired:** Pipelines, redirections, and logic operators.
- **Control Flow:** `if`, `while`, `for`, `loop`, and function support.
- **Cross-platform:** Native support for Windows and Unix-like systems.
- **Interactive:** Command history, tab completion, and rich prompts.
- **Job Control:** Manage background tasks with `bg`, `fg`, and `jobs`.

## Quick Start

Run the executable to enter the interactive shell:

```cerf
cerf
```

## Installation

Please go to [Releases](https://github.com/cerfsh/cerf/releases) to download the latest release (and read the guide)

Or see: [INSTALLATION.md](INSTALLATION.md)

Build from source:

```sh
# Clone the repository
git clone https://github.com/cerfsh/cerf.git

cd cerf
cargo build
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

## Scripting Best Practices

When writing scripts for Cerf, it is recommended to use the **namespaced command syntax** (e.g., `dir.cd` instead of `cd`).

Most common commands in Cerf are actually aliases to their namespaced counterparts:
- `cd` → `dir.cd`
- `pwd` → `dir.pwd`
- `echo` → `io.echo`
- `exit` → `sys.exit`

### Why use Namespaces?

Namespaced commands provide several advantages for scripting:
1. **No Alias Overrides:** Namespaced commands cannot be overridden by user aliases. This ensures that your script always uses the intended builtin command, regardless of the user's interactive configuration.
2. **Stability:** Scripts using namespaced commands are more resilient to changes in the shell's default alias mappings or user environment.

### Example

```cerf
# Recommendation: Use namespaced commands in scripts
dir.cd src
io.echo "Building project..."
sys.exec cargo build
```

## License

Licensed under the **Apache License 2.0**.
