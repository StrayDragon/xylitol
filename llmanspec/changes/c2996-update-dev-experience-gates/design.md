# Design

## Context

- `just qa` 热缓存约 70s，其中 workspace test 63s；CRAP CI 5–10 分钟，且对覆盖率抖动敏感。
- `just coverage-lcov` 只跑全 workspace；`just crap` 需要先有 `lcov.info`。
- TUI E2E 31 条只在 `just qa-e2e` 跑，CI 没有入口。
- `qa-spec-validate` 使用 strict，会把未勾 task / specs-not-landed 等 warning 升级为失败。

## Decision

### Q1 CRAP epsilon

`.cargo-crap.toml` 增加 `epsilon = 0.25`。cargo-crap 0.9 的 regression detector 以 `|Δ| ≤ epsilon` 视为 Unchanged；真实回归（新增分支导致 CRAP +1.0 以上）仍会红。保留 CI `fail-regression=true` / `fail-above=false`。

### Q2 `just test-changed`

脚本 `scripts/dev_test_changed.py`：

1. base = `git merge-base HEAD origin/HEAD`（缺省回退 `origin/main` / `HEAD~1`）。
2. 变更文件 = `git diff --name-only base`（含工作区）+ 未跟踪文件。
3. `cargo metadata` 解析每个 package 的 manifest 目录；文件归属取最长前缀包。
4. 反向依赖只取**直接** dependents（一跳），避免把整 workspace 拉进来；API 破坏仍能被直接调用方测试捕获。
5. 根级文件（`Cargo.toml`、`rust-toolchain.toml`、`.cargo/`、`tests/`、`llmanspec/specs/`、`scripts/`、`justfile`、`.github/`）→ 回退 `just test`。
6. 无 Rust 变更 → 打印提示并成功退出。
7. 用 nextest `-p ... --all-features --profile agent`；无 nextest 时 `cargo test -p ... --all-features`。

`just test-changed` 只是内环；push 前仍跑 `just qa`。

### Q3 `just crap-package`

```
just _cargo-llvm-cov-bin
cargo llvm-cov nextest -p <pkg> --all-features --profile ci --lcov --output-path target/crap-<pkg>.info
cargo crap -p <pkg> --lcov target/crap-<pkg>.info --top 25
```

无 baseline 比较，只做本地雷达；CI 仍跑全 workspace 回归。

### Q4 TUI E2E CI

新 workflow `tui-e2e.yaml`：

- 触发：`workflow_dispatch`、nightly `schedule`、PR 且带 `tui-e2e` label。
- 非阻塞：不加入 required checks。
- 步骤：checkout、Rust toolchain、rust-cache、apt 安装 tmux、`cargo test --test tui_e2e -- --ignored`。
- 不跑 `just qa`，避免重复 5 分钟编译；只补真终端层。

### Q5 validate 分层

- `just qa`：`llman-sdd validate --all`（只硬错误失败）。
- `just sdd` / `change finalize`：`--strict`（tasks-done / specs-landed 完成信号）。
- 理由：`qa` 是开发过程门禁，不应要求 in-flight change 的 tasks 全勾；收口仍由 finalize 卡死。

## Alternatives

- **Q1-B：`fail-regression=false`**：会放过真实回归；拒绝。
- **Q2-B：按 crate 路径直接 `cargo test -p`**：漏直接 dependents；拒绝。
- **Q4-B：PR 全量跑 E2E**：flaky 会阻塞合并；先 label/nightly 观察。
- **Q5-B：qa 继续 strict**：规划阶段就红门禁，已造成实际摩擦；拒绝。

## Risks

- `epsilon=0.25` 会放过极小真实回归（<0.25）；CRAP 分数变化通常以整数/半整数为主，风险可接受。
- `test-changed` 只跑一跳 dependents；跨层破坏由 `just qa` 兜底。
- E2E CI 可能 flaky；先非阻塞，观察后再决定是否升级。
- `qa` 非 strict 后，本地 warning 不再红；`just sdd` 与 finalize 仍严格。
