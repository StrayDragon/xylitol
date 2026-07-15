# Acceptance — c1005-update-app-tui-session-slash-rename

## 自动（agent 必跑）

```bash
cargo test --lib app::tui::harness::tests::h26 -- --nocapture
cargo test --lib app::tui::harness::tests::h27 -- --nocapture
cargo test --lib app::tui::harness::tests::h26b -- --nocapture
# 或整包相关：
cargo test --lib app::tui::harness -- --test-threads=1
llman sdd validate c1005-update-app-tui-session-slash-rename --strict --no-interactive
```

可选 PTY（真终端）：既有 session-tree PTY 仍有效；slash 入口改为 `/session-tree`（若用例字面钉旧名需同步）。

## 人类手测（最短）

前置：debug 构建、`--trust`、Fake 或可用模型。

1. 启动 TUI → 键入 `/` → 补全列表含 **`session-tree`** / **`session-fork`**，**不含** `tree` / `fork`。
2. `/session-tree` Enter → 树槽打开（同双 Esc）；Esc 关。
3. `/debug session-tree-branched`（或有 leaf 的会话）→ `/session-fork` Enter → 出现 Forked 类提示并切到 child。
4. `/tree` Enter → **unknown command**，树不开。
5. `/fork` Enter → **unknown command**，不 fork。

## 通过标准

- harness h26 / h27 / h26b 绿
- 手测 1–5 符合
- PI_DELTAS A02/A03 与 keybindings 文案一致
