# c96-refactor-architecture Tasks

- [ ] 抽象 model 解析：infra 层定义 `ResolvedModelSpec` DTO（defer - 大型重构）
- [ ] security wrap 移入 interface/bootstrap（defer - 需 bootstrap 提取）
- [ ] 提取 `src/interface/bootstrap.rs`（defer - 需全面理解三入口）
- [ ] Print/TUI/ACP 改用 bootstrap（defer - 联动上项）
- [ ] 审查 default features（defer - 需评估下游影响）
- [ ] 添加 `full` feature alias（defer - 联动上项）
- [x] 完善 README.md：项目描述、安装、快速开始、feature 矩阵、配置说明
- [x] 创建 `configs/example.yaml` 示例配置文件
- [x] 工具 limit（grep/find max_results 等）ToolsConfig 增加 limit 字段，消除 magic number
- [x] 清理 planner.rs/repeat.rs 中引用不存在的 docs/mmd/*.mmd
- [x] `just fmt && just lint && just test`
- [x] `llman sdd validate c96-refactor-architecture --strict --no-interactive`
