---
change_id: c440-refactor-tui-examples
---

# c440 Design — single example surface

## D1. Example policy

Examples in `packages/xylitol-tui` are not product code and should not be spread across `src/`. The example app stays in `examples/agent_demo.rs`; tests reuse it with:

```rust
#[path = "../examples/agent_demo.rs"]
mod agent_demo_example;
```

This keeps the boundary clean while still making the primary example testable.

## D2. Surface choice

The retained example should model the main coding-agent scenario:

- transcript/history
- tool execution events
- sidebar plan / changed files / recent tools
- bottom editor input
- small overlay surfaces (command palette / settings)

It should not try to be a component catalog.

## D3. Width policy

The example must prefer conservative width budgets over perfect density:

- all final rendered lines are clamped through a local `fit()` helper;
- example copy is ASCII-first to avoid mixing main-scenario acceptance with CJK-width edge cases;
- footer/status dynamic text is truncated.

This is acceptable because the example is a demo surface, not the reusable component API.

## D4. Acceptance target

The package-level acceptance path becomes:

- in-process: `agent_demo_test`
- PTY ignored smoke: `pty_agent_demo_submit_flow_survives_enter`
- tmux ignored smoke: existing example-start and styled-output checks, now pointed at `agent_demo`

## D5. Terminal support matrix

The package docs should state a narrow, explicit strategy:

- primary support: Linux/xterm-compatible terminals (`foot`, `wezterm`, `ghostty`, `alacritty`, `kitty`, `tmux`)
- baseline behavior delegated to `crossterm`
- advanced terminal protocol logic kept isolated in `terminal.rs`
- iTerm2/image/macOS-specific behavior not expanded unless there is a proven consumer
