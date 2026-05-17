# c10-add-config Tasks

- [ ] 添加 `minijinja`、`dotenvy`、`dirs` 依赖到 Cargo.toml
- [ ] 定义 `AppConfig` 及子结构体（ModelConfig, PlanningConfig, ExecutionConfig, SecurityConfig, HooksConfig, SessionConfig, RepeatDetectionConfig, PatchApplyConfig, CompactionConfig, ReviewConfig, SkillConfig, McpServerConfig, ValidationConfig）
- [ ] 实现 `serde` + `serde_yaml` 反序列化
- [ ] 实现模板渲染模块（minijinja sandbox，env.* + secret.* namespace，default filter，缺失变量错误报告）
- [ ] 实现 secret.env 加载（dotenvy 解析 + 文件权限警告）
- [ ] 实现配置目录发现（XDG / XYLITOL_CONFIG_DIR / XYLITOL_PROJECT_DIR / CWD walk，同时识别 `.xylitol/` 和 `.agents/`）
- [ ] 定义 `ConfigPaths` 结构体（`global_dir`, `project_dir`, `agents_dir`）并暴露给下游模块
- [ ] 实现 5 级配置加载（global base → global local → project base → project local → CLI --config）
- [ ] 实现深层合并逻辑（后者覆盖前者，安全规则仅收紧）
- [ ] 实现 `schemars` JSON Schema 生成 → `configs/config.schema.json`
- [ ] 实现 `jsonschema` 运行时校验 + 人可读错误报告
- [ ] 实现后置业务规则校验（模型 ID 引用完整性等）
- [ ] 创建 `.xylitol/secret.env.example` 模板文件
- [ ] 确保 `.gitignore` 包含 `config.local.yaml` 和 `secret.env` 模式
- [ ] 编写单元测试：模板渲染、secret 加载、目录发现、5 级合并、校验、默认值
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c10-add-config --strict --no-interactive`
