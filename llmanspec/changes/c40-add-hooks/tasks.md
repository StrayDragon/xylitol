# c40-add-hooks Tasks

- [ ] 定义 HookEvent 枚举（10+ 事件类型）
- [ ] 实现 HookDispatcher（事件分发 + 超时控制）
- [ ] 实现三级配置加载（全局/项目/用户会话）
- [ ] 实现钩子脚本执行（stdin JSON → stdout 控制指令）
- [ ] 实现 block/allow 控制指令解析
- [ ] 集成到 agent loop 和工具执行流程
- [ ] 编写测试：事件分发、配置覆盖、脚本执行
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c40-add-hooks --strict --no-interactive`
