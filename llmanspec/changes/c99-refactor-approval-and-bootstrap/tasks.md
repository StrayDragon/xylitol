# c99-refactor-approval-and-bootstrap Tasks

- [ ] 重构 ApprovalHub：wrapper 创建 channel 后注册，UI 通过 tx.send 响应
- [ ] ApprovalHub register 使用 Entry API 消除竞态
- [ ] 抽象 model 解析为 `ResolvedModelSpec` DTO（infra 层）
- [ ] 提取 `src/interface/bootstrap.rs`（理解三入口后抽象公共初始化）
- [ ] security wrap 移入 bootstrap
- [ ] Print/TUI/ACP 三入口改用 bootstrap
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c99-refactor-approval-and-bootstrap --strict --no-interactive`
