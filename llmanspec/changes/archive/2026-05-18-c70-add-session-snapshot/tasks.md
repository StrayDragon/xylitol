# c70-add-session-snapshot Tasks

- [x] 定义 Snapshot 数据结构（conversation, project_cognition, tool_call_log, config_fingerprint）
- [x] 实现核心操作（snapshot/restore/spawn/list/prune/diff/merge）
- [x] 实现增量快照（CoW 写时复制）
- [x] 实现 Zstandard 压缩
- [x] 实现 GC 策略（年龄/派生深度/标签白名单）
- [x] 实现上下文压缩 compaction（intra/manual/derive 三种策略）
- [x] 实现 codebase_graph 生成（基于 LSP summarize_symbols 或 tree-sitter 仓库摘要）
- [x] 实现蜂群派生（从快照派生新 agent 实例）
- [x] 编写测试（快照序列化、派生、合并、压缩）
- [x] 实现 fine-tune 交互式上下文编辑器（CLI 模式：树形展示 + 选择性编辑；Web 模式复用 c75 Monaco 基础设施，Phase 2 交付）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c70-add-session-snapshot --strict --no-interactive`
