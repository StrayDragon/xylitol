# Tasks: c235-fill-test-coverage

## Task 1: 创建测试策略文档
- [x] 创建 `docs/testing-strategy.md`，记录 BDD vs 单元测试的分界规则
      ```bash
      cargo test --lib  # 确保无回归
      cargo test --test bdd
      ```

## Task 2: core/error.rs — 错误类型单元测试
- [x] 在 `src/core/error.rs` 中添加 `#[cfg(test)] mod tests` 模块
- [x] 覆盖：`XyError` 的 Display（Provider / Tool / Session / Config / MaxIterations / Aborted）
- [x] 覆盖：`XyToolError` 的 Display（InvalidArgs / ExecutionFailed / PermissionDenied / Timeout / Aborted）
- [x] 覆盖：`From` 转换（`XyToolError` → `XyError`）
- [x] 校验：`cargo test --lib core::error` ✅ 15 passed

## Task 3: core/traits.rs — 核心 trait 契约测试
- [x] 在 `src/core/traits.rs` 中添加 `#[cfg(test)] mod tests` 模块
- [x] 覆盖：`XyToolCtx` 构造（new / with_cancel）、`CancellationToken` 关联
- [x] 覆盖：`ToolExecutionMode` 默认值、PartialEq、序列化
- [x] 覆盖：Mock 实现验证 `XyTool` trait 方法可正常调用（name / description / parameters_schema / execute）
- [x] 校验：`cargo test --lib core::traits` ✅ 9 passed

## Task 4: core/types.rs — 核心数据类型测试
- [x] 在 `src/core/types.rs` 中添加 `#[cfg(test)] mod tests` 模块
- [x] 覆盖：`XyChunk` 各变体构造（TextDelta / ThinkingDelta / FunctionCall / Done）
- [x] 覆盖：`ThinkingLevel` 序列化/反序列化（serde round-trip）
- [x] 覆盖：`ThinkingLevel::clamp` 行为（模型不支持 thinking 时→Off）
- [x] 覆盖：`ThinkingLevel::as_str` 输出
- [x] 覆盖：`ModelMeta` 构造字段
- [x] 校验：`cargo test --lib core::types` ✅ 15 passed

## Task 5: core/model.rs — 模型配置测试
- [x] 在 `src/core/model.rs` 中添加 `#[cfg(test)] mod tests` 模块
- [x] 覆盖：`ModelKind::from_provider_name` 解析（openai / anthropic / fake / 未知）
- [x] 覆盖：`ModelKind::provider_name` 输出
- [x] 覆盖：`ModelConfig` 构建与 `provider_name()` 方法
- [x] 覆盖：`default_context_window_for` 返回值
- [x] 校验：`cargo test --lib core::model` ✅ 13 passed

## Task 6: agent/queue.rs — MessageQueue 单元测试
- [x] 在 `src/agent/queue.rs` 中添加 `#[cfg(test)] mod tests` 模块
- [x] 覆盖：new() 空队列
- [x] 覆盖：push / drain 基本推拉
- [x] 覆盖：steer / drain_steering 优先消息
- [x] 覆盖：follow_up / drain_follow_up
- [x] 覆盖：混合 steering + follow_up drain 顺序
- [x] 覆盖：clear 返回所有消息
- [x] 覆盖：pending_count / has_pending 在空/非空队列状态
- [x] 覆盖：多次 drain 返回空
- [x] 校验：`cargo test --lib agent::queue` ✅ 13 passed

## Task 7: agent/retry.rs — RetryState 状态机测试
- [x] 已有测试覆盖（retryable patterns / exponential backoff / abort）
- [x] 校验：`cargo test --lib agent::retry` ✅ 已有并继续通过

## Task 8: agent/commands.rs — 命令解析测试
- [x] 已有测试覆盖（SlashCommandInfo / is_slash_command / find_command / args / TUI 判定）
- [x] 校验：`cargo test --lib agent::commands` ✅ 已有并继续通过

## Task 9: Session 子组件测试
- [x] 在 `src/agent/model_manager.rs` 中添加 `#[cfg(test)]` 测试
      - 覆盖：cycle_forward 循环、select_model、set_thinking_level
- [x] 在 `src/agent/tool_manager.rs` 中添加 `#[cfg(test)]` 测试
      - 覆盖：new / tool_registry 访问、set_active_tools 过滤
- [x] 在 `src/agent/skill_manager.rs` 中添加 `#[cfg(test)]` 测试
      - 覆盖：new / skills 访问、register_commands
- [x] 校验：`cargo test --lib` ✅ model_manager 10 passed, tool_manager 6 passed, skill_manager 6 passed

## Final Verification
- [x] 全量测试通过
      ```bash
      cargo test --lib 2>&1 | tail -5  # 509 passed (421 original + 88 new)
      cargo test --test bdd 2>&1 | tail -5  # 79 BDD 无回归
      ```
- [x] `docs/testing-strategy.md` 文档完整性检查
