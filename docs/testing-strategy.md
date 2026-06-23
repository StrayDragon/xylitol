# Testing Strategy

> 本文档定义 xylitol 项目中 **BDD 场景** 与 **单元测试** 的职责分界，确保测试覆盖不重复、不遗漏。

## 分层结构

参照 `../pi` 项目的 TypeScript vitest 分层：

| 层级 | xylitol 对应 | 框架 | 覆盖目标 |
|---|---|---|---|
| **纯模块测试** | `#[cfg(test)]` in `src/` | cargo built-in | 纯算法、数据结构契约、组件内状态机 |
| **集成/BDD 测试** | `tests/bdd.rs` + `tests/features/` | rstest-bdd | 端到端编排、CLI 行为、跨组件交互 |
| **E2E provider**（暂按需） | 单独 crate/CI | — | 真实 provider API 验证 |

## BDD 场景（不重复写单元测试）

以下行为**只通过 BDD 场景覆盖**，不重复写在源文件单元测试中：

- **Agent 循环 turn 流程**：prompt → model → tool → result → loop
- **工具执行完整路径**：参数验证 → 执行 → 输出格式化（每个工具一条场景）
- **Session 持久化读写**：创建/加载/分叉/树导航/导出
- **CLI 命令分发**：`/model`、`/compact`、`/export` 等 slash 命令
- **错误消息输出**：工具缺少参数、权限拒绝、沙箱拦截
- **Hook 执行序列**：pre/post hook、hook 修改参数、超时
- **配置加载**：三层合并、环境变量插值、provider base_url

## 单元测试（不重复写 BDD）

以下行为**只通过 `#[cfg(test)]` 单元测试覆盖**，不使用 BDD 全链路验证：

- **纯数据结构**：序列化/反序列化 round-trip、Display、From 转换
- **无副作用算法**：cut point 检测、token 估算
- **组件内状态机**：MessageQueue 推/拉/清空、RetryState 转换
- **字符串解析**：slash 命令检测、模板参数替换、bash bang 前缀
- **枚举变体行为**：ThinkingLevel clamp、StopReason 匹配
- **配置数据模型**：ModelKind 解析、ModelConfig 构建、default_context_window

## 重叠区域（各有不同角度）

当某个行为已在 BDD 覆盖时，单元测试**只测该路径用到的底层函数边界**：

- BDD 验证了 "agent 正确处理工具调用" → 单元测试只测工具参数 schema 构造和结果格式化
- BDD 验证了 "session 创建和加载" → 单元测试只测 EntryBase 构造和 timestamp 格式
- **不做**：用 BDD 验证底层算法细节，或用单元测试验证完整编排流程

## 文件组织约定

1. **`core/`** 中每个子模块必须有一个 `#[cfg(test)] mod tests`
2. **`agent/`** 中纯逻辑文件（`queue.rs`、`retry.rs`、`commands.rs`、`templates.rs`、`config_value.rs`）的测试写在各自文件内
3. **Session 子组件**（`model_manager.rs`、`tool_manager.rs`、`skill_manager.rs`）的测试写在各自文件内
4. **BDD feature 文件**在 `tests/features/`，step 实现在 `tests/bdd.rs`
5. 测试不要依赖 CWD；需要 fixture 文件时使用 `env!("CARGO_MANIFEST_DIR")`

## 快速校验命令

```bash
# 全量回归
cargo test --lib
cargo test --test bdd -- --test-threads=1
```
