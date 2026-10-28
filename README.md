<div align="center">

<img src="script/warp.svg" alt="Warp Logo" width="64" height="64" />

# Warp

**A fast, GPU-accelerated terminal built in Rust — designed to work like a modern IDE.**

[![Rust 1.92+](https://img.shields.io/badge/rust-1.92%2B-orange.svg?style=flat-square&logo=rust)](rust-toolchain.toml)
[![License: AGPL-3.0](https://img.shields.io/badge/license-AGPL--3.0-blue.svg?style=flat-square)](Cargo.toml)
[![Platforms: macOS | Linux | Windows](https://img.shields.io/badge/platform-macOS%20%7C%20Linux%20%7C%20Windows-lightgrey.svg?style=flat-square)](#supported-platforms)
[![Build System: Cargo & Nextest](https://img.shields.io/badge/build-cargo%20%7C%20nextest-green.svg?style=flat-square)](#development-workflow)

[Features](#key-features) • [Quickstart](#getting-started) • [Architecture](#architecture--workspace-layout) • [Development](#development-workflow) • [Contributing](#contributing) • [License](#license)

</div>

---

## Overview

Traditional terminal emulators have treated input and output as an unorganized stream of character cells for decades. **Warp** reimagines the terminal experience from the ground up:

- **Commands as Blocks**: Every executed command and its output are encapsulated into interactive, isolated units that you can navigate, copy, filter, or share.
- **IDE-Grade Editor**: Input is treated as an editor buffer rather than a dumb text stream. Click anywhere to move your cursor, select text with your mouse or keyboard, use multiple cursors, and navigate with Vim keybindings.
- **Hardware-Accelerated Performance**: Built completely in Rust on top of a custom WebGPU/Metal/Vulkan rendering pipeline (`warpui`), delivering consistent 120 FPS rendering without CPU lag.
- **Extensible & AI-Native**: Integrated Model Context Protocol (MCP) tools, natural language command synthesis, and agentic workflows built directly into the session loop.

---

## Key Features

### 🧱 Block-Based Command Execution
- Isolates commands and their stdout/stderr into distinct, navigable blocks.
- Filter outputs in real time, search prior runs, or copy command outputs with a single click.
- Native status indicators showing exit codes, execution duration, and timestamp metadata.

### ✍️ Modern Text Editing Input
- True text editing primitives: mouse selection, word boundaries, undo/redo stacks, and auto-closing delimiters.
- Built-in Vim keybindings (`normal`, `insert`, and `visual` modes).
- Syntax highlighting and bracket matching powered by native syntax trees.

### ⚡ Context-Aware Autocompletion
- Sub-millisecond parameter suggestions and man-page documentation lookup as you type.
- Community and custom command signatures for popular tools (`git`, `docker`, `kubectl`, `cargo`, etc.).
- Intelligent history search that filters past commands by context and frequency.

### 🤖 AI Assistance & Model Context Protocol (MCP)
- Direct terminal AI query integration to translate natural language into commands.
- Built-in MCP client runtime (`crates/mcp`) supporting external agents, tool servers, and automated diagnostics.
- Workflows and parameterized templates for repeatable team operations via Warp Drive.

---

## Architecture & Workspace Layout

Warp is structured as a modular Rust workspace consisting of more than 80 decoupled crates:

```
warp-lol/
├── app/                        # Main desktop application, macOS bundle packaging, platform glue
│   ├── src/                    # Window bootstrap, session coordinator, event loop
│   ├── assets/                 # Embedded fonts, themes, icons, and platform resources
│   └── DockTilePlugin/         # macOS native dock tile extension
├── crates/
│   ├── warpui/                 # Custom GPU UI library (Metal / Vulkan / DirectX backends)
│   ├── warpui_core/            # Primitives: layout, drawing commands, color spaces, widgets
│   ├── warp_terminal/          # PTY lifecycle, VT100/ANSI parsing, terminal state machine
│   ├── editor/                 # Text input buffer, multi-cursor engine, syntax tokenization
│   ├── ai/                     # AI assistant orchestration, streaming client, and prompts
│   ├── mcp/                    # Model Context Protocol client and tool management
│   ├── computer_use/           # Automation primitives and agent execution environment
│   ├── warp_completer/         # Autocompletion engine, fuzzy matcher, signature index
│   ├── command-signatures-v2/  # Spec registry for CLI completions and parameter hints
│   ├── sum_tree/               # Concurrent, B-tree based interval storage for text & blocks
│   └── vim/                    # Vim keybinding engine and modal state machine
├── resources/                  # Bundled skill definitions, Linux desktop entries, templates
├── script/                     # Cross-platform build, bootstrap, and verification scripts
├── rust-toolchain.toml         # Pinned Rust compiler toolchain (1.92.0)
└── Cargo.toml                  # Workspace manifest and dependencies
```

---

## Getting Started

### Prerequisites

Ensure you have the following installed on your machine:

- **Rust**: Pinned to `1.92.0` (managed automatically via `rustup` reading [rust-toolchain.toml](rust-toolchain.toml)).
- **macOS**:
  - Xcode Command Line Tools (`xcode-select --install`) or full Xcode installation.
  - Python 3 (`python3`) and `pkg-config`.
- **Linux**:
  - `build-essential`, `pkg-config`, `libx11-dev`, `libxcb1-dev`, `libssl-dev`, and a Vulkan/OpenGL driver.
- **Windows**:
  - Visual Studio 2022 with C++ Build Tools and Windows SDK.

---

### Step 1: Bootstrap the Environment

Run the automated bootstrap script for your platform to install required development tools, linters, and dependencies:

```bash
# Clone the repository
git clone https://github.com/MohdSaleh/warp-lol.git
cd warp-lol

# Run the cross-platform bootstrap script
./script/bootstrap
```

> **Note**: On macOS, `./script/bootstrap` configures Xcode developer tools and installs necessary formatting and signing helpers.

---

### Step 2: Build and Run

To launch Warp in local development mode:

```bash
./script/run
```

- On **macOS**, `./script/run` compiles the workspace, packages it into `target/debug/bundle/osx/Warp.app`, signs it for local development, and launches the app.
- On **Linux** and **Windows**, it compiles and executes the `warp-oss` binary target with active GPU acceleration.

To run with release optimizations:

```bash
./script/run --release
```

---

## Development Workflow

### Presubmit Checks

Before submitting changes, run the local presubmit pipeline. This script mirrors our continuous integration checks:

```bash
./script/presubmit
```

This verifies:
1. **Formatting**: Runs `./script/format --check` across Rust, WGSL shaders, and C/Objective-C files.
2. **Clippy Lints**: `cargo clippy --workspace --all-targets --tests -- -D warnings`.
3. **Module Structure**: Verifies no inline test modules exist in production crates.
4. **Unit & Integration Tests**: Runs tests using `cargo-nextest`.

### Code Formatting

Auto-format all code in the workspace:

```bash
./script/format
```

### Running Tests

We use [cargo-nextest](https://nexte.st/) for fast, parallelized test execution:

```bash
# Run all workspace unit and integration tests
cargo nextest run

# Run tests for a specific crate (e.g. editor or completer)
cargo nextest run -p editor
cargo nextest run -p warp_completer
```

---

## Supported Platforms

| Platform | Tier | Architecture | Packaging |
| :--- | :---: | :---: | :--- |
| **macOS** | Tier 1 | Apple Silicon (`arm64`), Intel (`x86_64`) | Native `.app` Bundle, DMG |
| **Linux** | Tier 1 | `x86_64`, `aarch64` | AppImage, `.deb`, `.rpm`, Arch PKGBUILD |
| **Windows** | Tier 2 | `x86_64`, `arm64` | Inno Setup Installer (`.exe`), Standalone Binary |

---

## Contributing

We welcome contributions to bug fixes, performance enhancements, and new command signatures!

1. **Fork & Branch**: Create a descriptive feature branch from `main`:
   ```bash
   git checkout -b feat/your-feature-name
   ```
2. **Commit Early**: Write clear, imperative commit messages.
3. **Verify Locally**: Ensure `./script/presubmit` passes cleanly with zero warnings.
4. **Open a PR**: Submit a Pull Request detailing what was changed and including screenshots or reproduction steps where applicable.

---

## License

This project is licensed under the **GNU Affero General Public License v3.0** (`AGPL-3.0-only`). See [Cargo.toml](Cargo.toml) for package license metadata.

