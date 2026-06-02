---
name: rust-cli-tui-developer
description: '当需要用 Rust 构建命令行工具或终端 UI（clap 参数解析、inquire 交互式提示、ratatui TUI 事件循环/布局/组件）时使用。Use when building Rust command-line apps or terminal UIs: clap-based CLI parsing, inquire-based interactive prompts, or ratatui-based TUI event loops/layout/widgets. Keywords: Rust, CLI, command line, 命令行, TUI, terminal UI, 终端UI, clap, subcommand, args, help, inquire, prompt, interactive, ratatui, crossterm, event loop, widget, layout, 发布'
allowed-tools: Read Write Edit Glob Grep Bash
metadata:
  clap_version: "4.5"
  inquire_version: "0.7"
  ratatui_version: "0.28"
  clap_repo: "https://github.com/clap-rs/clap"
  inquire_repo: "https://github.com/mikaelmello/inquire"
  ratatui_repo: "https://github.com/ratatui-org/ratatui"
---

# Rust CLI/TUI Developer

## Overview

Build modern Rust CLIs and TUIs with a clear progression: **CLI first**, then add prompts or a full-screen TUI only when needed.

## Decision guide

- **CLI only** → `clap`
- **CLI + interactive prompts** → `clap` + `inquire`
- **Full TUI** → `ratatui` (+ `crossterm`) (often still keep `clap` for non-interactive mode)

## Workflow（Recommended）

1. **Pick interaction mode** (CLI / prompts / TUI) and keep a non-interactive path when possible.
2. **Start from a bundled example**
   - Clap: `assets/examples/clap/`
   - Ratatui (UI recordings & assets): `assets/examples/ratatui/`
3. **Implement the minimal “happy path”** (parse args → run one command) before adding options.
4. **Add structure**
   - Subcommands for major actions
   - Typed config structs; explicit error handling
   - Logging and exit codes
5. **Test**
   - CLI parsing tests (`try_parse_from`)
   - Unit tests for pure logic (keep UI thin)
6. **Release**
   - Use `references/RELEASE_DISTRIBUTION.md` for release profile and CI patterns

## References index

- Clap patterns: `references/README_CLAP.md`
- Inquire patterns: `references/README_INQUIRE.md`
- Inquire key bindings: `references/INQUIRE_KEY_BINDINGS.md`
- Ratatui patterns: `references/README_RATATUI.md`
- Ratatui architecture: `references/RATATUI_ARCHITECTURE.md`
- Source/setup notes (optional): `references/SOURCE_SETUP.md`
- Release & distribution: `references/RELEASE_DISTRIBUTION.md`

## Guardrails

- Keep core logic decoupled from UI rendering; UI should be a thin shell.
- Prefer explicit state machines for TUIs; avoid hidden globals.
- For TUIs, handle terminal resize and ensure clean teardown on panic/exit.

## Done checklist

- A minimal command works end-to-end before adding flags/subcommands.
- UI/prompt code is isolated and testable logic lives outside UI layers.
- Release build and basic CI plan exist (even if not implemented yet).
