# c45-add-lsp-layer Tasks

- [x] 封装 lspz AgentPool/AgentHandle
- [x] 实现三种初始化策略（懒加载/预初始化/按需预热）
- [x] 实现 LSP server 子进程生命周期管理（spawn + kill_on_drop）
- [x] 封装 10+ 查询方法 + 3 个同步方法 + 3 个重构操作
- [x] 实现 TOON 文本压缩输出
- [x] 实现 feature flag 门控（feature = "infra-lsp"）
- [x] 编写测试（mock transport、协议消息解析）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c45-add-lsp-layer --strict --no-interactive`
