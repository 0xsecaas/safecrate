# Safecrate: Safely Inspect Untrusted Code

Safecrate is a CLI tool that lets you open and build untrusted source code in a secure, isolated Docker container. It prevents risky commands from running on your machine by providing a sandboxed environment with Rust, Neovim, and Rust Analyzer pre-configured.

⚠️ **Security Notice:** For maximum security, run Safecrate inside a VM. Docker isolation is strong but not infallible against kernel or daemon exploits.

## Quick Start

```bash
# 1. Initialize the sandboxed environment
safecrate init

# 2. Open an untrusted project
safecrate open /path/to/untrusted_code

# 3. Resume a previous session
safecrate resume /path/to/untrusted_code

# 4. Clean up the container
safecrate remove /path/to/untrusted_code
```

## Features

- **Isolate Untrusted Code:** Open any project in a sandboxed container to prevent access to your host system.
- **Pre-configured for Rust:** Comes with Neovim and Rust Analyzer for a ready-to-use development environment.
- **Customizable:** Use your own Dockerfile for other languages or tools (`safecrate init --dockerfile_PATH`).
- **Control Execution:** Override the default command, keep containers alive, or disable networking.

```bash
# Example: Open a shell with no network access
safecrate open UNTRUSTED_DIR --cmd "bash" --no-network
```

Safecrate works by mounting the project directory into a Docker container, so all build tools and code analysis run in isolation, keeping your system safe.
