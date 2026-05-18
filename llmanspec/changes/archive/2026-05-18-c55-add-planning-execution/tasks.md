# c55-add-planning-execution Tasks

- [x] 定义 Planner/Executor/Validator 角色 trait
- [x] 实现 Planner（任务分解为 JSON 步骤计划）
- [x] 实现 Executor（按计划逐步执行）
- [x] 实现 Validator（编译/lint/测试验证）
- [x] 实现模型路由（Planner/Executor 绑定不同模型）
- [x] 实现 fallback（自动切换到配置的备用模型 OpenAI↔Anthropic）
- [x] 实现系统提示词模板（architect/editor）
- [x] 编写测试（mock LLM 规划 + 执行流程）
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c55-add-planning-execution --strict --no-interactive`
