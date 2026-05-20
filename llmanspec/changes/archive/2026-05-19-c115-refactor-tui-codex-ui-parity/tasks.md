# c115-refactor-tui-codex-ui-parity Tasks

- [x] **1 — 调研 Codex 的布局/渲染骨架**：阅读 `../codex/codex-rs/tui/src/chatwidget/*`、`bottom_pane/*` 与 `styles.md`，整理必须复刻的布局结构与渲染约束（无 Boxes、footer 逻辑、prefix/wrap）。
  - Evidence: `../codex/codex-rs/tui/src/chatwidget/rendering.rs`, `../codex/codex-rs/tui/src/bottom_pane/footer.rs`, `../codex/codex-rs/tui/styles.md`

- [x] **2 — 设计 xylitol 的 Codex-style TUI 架构**：在 `design.md` 写清楚新组件树（transcript + bottom pane + overlay stack）、事件路由（app/global vs composer vs overlay）以及与 xylitol AgentLoop/ApprovalHub/ReviewEngine 的接线点。

- [x] **3 — 重写 UI 布局壳（不动 review UI）**：移除现有 `Chat/Tools/Input/StatusBar` 盒子布局渲染路径，新增 Codex-style root 组件与 bottom pane；保留并继续使用 review overlays（Approval/DiffPreview）。
  - Verify: `cargo test --features ui-tui`

- [x] **4 — Footer/statusline 迁移**：用 Codex-style footer 替换 `StatusBar`，支持 running、queue、shortcuts/backtrack hints 与最小上下文信息（model/session）。
  - Verify: 手动 + `cargo test --features ui-tui`

- [x] **5 — Transcript 渲染风格对齐**：把 transcript 渲染从“role label + box”迁移为 Codex-style cell/prefix/wrap（例如用户消息 `› `，assistant streaming caret），并保留 tool card/thinking 的可折叠能力。
  - Verify: `cargo test --features ui-tui`

- [x] **6 — Raw output 真值实现**：实现真正的 raw output toggle（Alt+R）——切换 transcript 与 composer 的渲染策略为更适合终端复制的纯文本模式，而不是仅刷新。
  - Verify: 手动 + `cargo test --features ui-tui`

- [x] **7 — Spec delta 完善 + 校验**：补齐 `llmanspec/changes/c115-.../specs/tui-interface/spec.md`（新增 ops/scenarios 已有基础，按实现补充更多），并跑严格校验。
  - Verify: `llman sdd validate c115-refactor-tui-codex-ui-parity --strict --no-interactive`

- [x] **8 — 回归验证**：`cargo fmt` + `cargo clippy` + `cargo test --all-features`（或项目推荐的 `just qa`）。
