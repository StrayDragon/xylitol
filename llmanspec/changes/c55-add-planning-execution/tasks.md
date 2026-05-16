# c55-add-planning-execution Tasks

- [ ] 定义 Planner/Executor/Validator 角色 trait
- [ ] 实现 Planner（任务分解为 JSON 步骤计划）
- [ ] 实现 Executor（按计划逐步执行）
- [ ] 实现 Validator（编译/lint/测试验证）
- [ ] 实现模型路由（Planner/Executor 绑定不同模型）
- [ ] 实现 fallback（自动切换到配置的备用模型 OpenAI↔Anthropic）
- [ ] 实现系统提示词模板（architect/editor）
- [ ] 编写测试（mock LLM 规划 + 执行流程）
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c55-add-planning-execution --strict --no-interactive`
