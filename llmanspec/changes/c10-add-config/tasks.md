# c10-add-config Tasks

- [ ] 定义 `AppConfig` 及子结构体（ModelConfig, PlanningConfig, ExecutionConfig, SecurityConfig, HooksConfig, SessionConfig, RepeatDetectionConfig, PatchApplyConfig, CompactionConfig, ReviewConfig, SkillConfig, McpServerConfig, ValidationConfig）
- [ ] 实现 `serde` + `serde_yaml` 反序列化
- [ ] 实现 `schemars` JSON Schema 生成 → `configs/config.schema.json`
- [ ] 实现三级配置加载（全局 ~/.config/xylitol/ + 项目 .xylitol/ + CLI --config）
- [ ] 实现深层合并逻辑（后者覆盖前者）
- [ ] 实现 `jsonschema` 运行时校验 + 错误报告
- [ ] 编写单元测试：解析、合并、校验、默认值
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c10-add-config --strict --no-interactive`
