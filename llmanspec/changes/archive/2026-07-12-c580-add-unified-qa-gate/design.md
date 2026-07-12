# Design — c580 unified qa gate

## 组成

```text
just qa
  ├─ fmt-check
  ├─ lint          (clippy --all-features -D warnings)
  ├─ test          (nextest/cargo --all-features，含 workspace)
  ├─ test-tui      (显式 packages/xylitol-tui 层 1–4)
  ├─ doc-check
  ├─ check-tui-tokens
  └─ prek run --all-files

just qa-e2e = just qa && just test-tui-e2e
```

## 为何 test-tui 仍显式

workspace `test` 通常已覆盖成员 crate，但满闸清单需要**人类/agent 可读的「包 TUI harness 必过」一步**；与 `test-tui-harness` skill 对齐，避免只跑主 crate 时误以为 TUI 已验。

## 与 E2E 的关系

第 5 层（`tests/tui_e2e/`）依赖真实 PTY / 可选 tmux，规格要求 `#[ignore]` + 专用 recipe。默认 `qa` 保持可在无 tmux 的 CI/沙箱复现；需要协议/真终端时跑 `qa-e2e`。
