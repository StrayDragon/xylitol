# c96-refactor-architecture Tasks

- [ ] 抽象 model 解析：infra 层定义 `ResolvedModelSpec` DTO，agent 层 `From<>` 转换
- [ ] security wrap 移入 interface/bootstrap，而非 agent/tools
- [ ] 提取 `src/interface/bootstrap.rs`：统一 ToolRegistry + MCP + Security + Profile 构建
- [ ] Print/TUI/ACP 入口改用 `bootstrap::build_runtime(config)` 替代各自重复逻辑
- [ ] 审查 default features：将 LSP/DAP/session/planning 移出 default，保留核心
- [ ] 添加 `full` feature alias 聚合全部
- [ ] 完善 README.md：项目描述、安装、快速开始、feature 矩阵、配置说明
- [ ] 创建 `configs/example.yaml` 示例配置文件
- [ ] 工具 limit（grep/find max_results 等）从 ToolsConfig 读取，消除 magic number
- [ ] 清理 planner.rs/repeat.rs 中引用不存在的 docs/mmd/*.mmd
- [ ] `just fmt && just lint && just test`
- [ ] `llman sdd validate c96-refactor-architecture --strict --no-interactive`
