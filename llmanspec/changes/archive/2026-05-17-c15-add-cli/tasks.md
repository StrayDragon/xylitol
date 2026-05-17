# c15-add-cli Tasks

- [x] 定义 clap derive 结构体（CliArgs）含 mode/config/project/model/yolo 参数
- [x] 实现 RunMode 枚举（Print/Interactive/Acp）和分派逻辑
- [x] 集成配置加载（调用 infra::config）
- [x] 重写 main.rs：解析参数 → 加载配置 → 分派到模式
- [x] 编写测试：参数解析、默认值、模式选择
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c15-add-cli --strict --no-interactive`
