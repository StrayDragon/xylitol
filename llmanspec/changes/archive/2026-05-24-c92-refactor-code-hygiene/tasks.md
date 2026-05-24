# c92-refactor-code-hygiene Tasks

- [x] 移除 `src/lib.rs:5` 的 `#![allow(dead_code)]`；WIP 模块添加局部允许（119→51 warnings）
- [ ] 提取 `src/agent/tools/args.rs` helpers（defer - 较大重构）
- [ ] 将 7 个工具的参数解析迁移到新 helper（defer - 依赖上项）
- [ ] 拆分 `src/interface/tui/app.rs`（defer - 1600+ 行拆分需专项重构）
- [ ] 统一 session/lsp 等内部模块的可见性为 `pub(crate)`（defer - 与 c96 联动）
- [x] `--project` flag 标记 `#[arg(hide = true)]`
- [x] `--yolo` flag 标记 `#[arg(hide = true)]`
- [ ] 统一错误类型约定并更新 AGENTS.md（defer - 架构决策）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c92-refactor-code-hygiene --strict --no-interactive`
