---
depends_on: [c50-add-security]
---

# c96-refactor-architecture

## Why

可维护性审查发现多个影响长期演进的架构问题：

1. **agent ↔ infra 循环依赖**：`AppConfig::resolve_model()` 返回 `agent::model::ModelConfig`，`ToolRegistry::wrap_with_security()` 反向依赖 `infra::security`，破坏分层边界
2. **大量功能已实现但未接入主流程**：PlanningOrchestrator、LspPool、SnapshotManager、DAP 均无生产路径调用
3. **启动逻辑三处重复**：Print/TUI/ACP 各自重复 ToolRegistry 构建、MCP 注册、SecurityEngine 包装
4. **Default features 过重**：12 项全开，最小构建不可用，编译时间与 binary 体积虚高
5. **README 仅一行**：新贡献者 onboarding 成本高，缺少示例 config
6. **Provider 扩展封闭**：`ModelKind` 硬编码两家 provider，每增一家需改三处
7. **配置默认值分散**：工具层 magic number 与 `AppConfig` 默认值不一致
8. **文档引用缺失资源**：planner/repeat 模块引用不存在的 mermaid 图表

## What Changes

1. 打破循环依赖：model 解析结果抽象为 infra 层 DTO，security 包装移入 composition root
2. 未接入模块移出 default features；或建立 CLI composition root 完成接线
3. 提取 `interface/bootstrap.rs` 统一运行时构建
4. Default features 收窄为核心 subset；提供 `full` feature 别名
5. 完善 README（项目定位、快速开始、feature 矩阵、环境变量）
6. 添加示例 `configs/example.yaml`
7. 统一配置默认值 SSOT（工具 limit 从 config 读取）
8. 清理缺失文档引用

## Capabilities

- `workspace-structure`: 项目结构与架构分层

## Impact

- Feature 变更可能影响下游 CI/嵌入构建
- Bootstrap 重构需同步更新三个 mode 的入口
- README/文档变更为低风险高收益
