---
depends_on: []
title: 收敛开发体验门禁与本地反馈
needs_specs_change: false
---

# 收敛开发体验门禁与本地反馈

## Why

- CRAP 回归门禁对覆盖率抖动过敏：近期 CI 出现 `+0.03` 假红，单次等待 5–10 分钟。
- 本地 CRAP 反馈只能跑全 workspace coverage，缺少单包入口；内环测试也只能跑全量 63s。
- 真终端 TUI E2E 只在本地 `just qa-e2e`，CI 完全没跑，TUI 回归只能靠自觉。
- `just qa` 用 `validate --all --strict`，规划中/进行中的 change 会因未勾 task 直接红门禁。

## What Changes

- **Q1**：`.cargo-crap.toml` 增加 `epsilon = 0.25`，容忍覆盖率抖动，真实回归（通常 ≥1.0）仍红。
- **Q2**：新增 `scripts/dev_test_changed.py` 与 `just test-changed`：按变更文件映射到包 + 直接反向依赖，跑 nextest；根级/规格/测试目录变更回退全量。
- **Q3**：新增 `just crap-package <pkg>`：单包 `cargo llvm-cov nextest` + `cargo crap -p`，只做本地雷达。
- **Q4**：新增 `.github/workflows/tui-e2e.yaml`：`workflow_dispatch` + nightly + `tui-e2e` label，非阻塞；装 tmux 后跑 `cargo test --test tui_e2e -- --ignored`。
- **Q5**：`qa-spec-validate` 改 `llman-sdd validate --all`（非 strict）；`just sdd` 与 `change finalize` 仍 strict。
- 更新 `xylitol-dev` skill `references/qa-gates.md` 记录上述口径。

## Capabilities

- 无产品行为合约变更；只改开发门禁与反馈工具。

## Impact

- `.cargo-crap.toml`
- `justfile`
- `scripts/dev_test_changed.py`
- `.github/workflows/tui-e2e.yaml`
- `.agents/skills/xylitol-dev/references/qa-gates.md`

## Out of Scope

- 不改产品代码、specs、协议或 TUI 行为。
- 不让 E2E 成为 PR 阻塞门禁（先观察 flaky）。
- 不把 CRAP 纳入 `just qa`（仍走 CI / 本地显式）。
- 不放宽 `fail-above` 或阈值。
