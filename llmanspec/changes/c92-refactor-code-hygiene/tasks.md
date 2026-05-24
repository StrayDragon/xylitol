# c92-refactor-code-hygiene Tasks

- [ ] 移除 `src/lib.rs:5` 的 `#![allow(dead_code)]`；修复暴露的 dead code warnings
- [ ] 提取 `src/agent/tools/args.rs`：`require_str()`、`optional_str()`、`require_i64()` 等 helpers
- [ ] 将 7 个工具的参数解析迁移到新 helper
- [ ] 拆分 `src/interface/tui/app.rs`：提取 clipboard/editor/session/review 子模块
- [ ] 统一 session/lsp 等内部模块的可见性为 `pub(crate)`
- [ ] 实现 `--project` flag（传入 ConfigPaths/工作目录）或标记 `#[arg(hide = true)]`
- [ ] 实现 `--yolo` flag（跳过 security/approval）或标记 `#[arg(hide = true)]`
- [ ] 统一错误类型约定并更新 AGENTS.md
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c92-refactor-code-hygiene --strict --no-interactive`
