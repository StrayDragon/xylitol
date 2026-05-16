# c15-add-cli Tasks

- [ ] 定义 clap derive 结构体（CliArgs）含 mode/config/project/model/yolo 参数
- [ ] 实现 RunMode 枚举（Print/Interactive/Json）和分派逻辑
- [ ] 集成配置加载（调用 infra::config）
- [ ] 重写 main.rs：解析参数 → 加载配置 → 分派到模式
- [ ] 编写测试：参数解析、默认值、模式选择
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c15-add-cli --strict --no-interactive`
