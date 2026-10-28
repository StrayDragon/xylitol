# Tasks

- [x] 1. **specs**（spec）：`domain-compaction` 增状态条目保留规则与 BDD 场景；`app-tui` 增恢复会话初始化 ModeStrip 规则。测：`llman-sdd validate --all --strict`。
- [x] 2. **compaction**（plugin）：压缩后重追加缺失的 `plan_mode` / `plan_doc` / `plan_ref` / `workflow` / `todo_checkpoint` 快照。测：BDD 压缩后仍为 plan。
- [x] 3. **host**（serve）：切换/新建会话后按新会话最新 `plan_mode` 发布 `PlanModeChanged`。测：serve 单测载荷。
- [x] 4. **门禁**（repo）：`just qa` 全绿；tmux 手工复验 plan 模式与 workflow 不再无限续跑。测：`just qa` + tmux。
