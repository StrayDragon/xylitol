# Design: c235-fill-test-coverage

## 测试策略决策 — BDD vs 单元测试分界

参照 `../pi` 项目的 TypeScript vitest 分层，xylitol（Rust）对应映射：

| pi 层级 | pi 示例文件 | xylitol 对应 | 测试框架 | 适合场景 |
|---|---|---|---|---|
| 纯模块测试 | `ai/test/abort.test.ts` | `#[cfg(test)]` in `src/` | 内置测试运行器 | 纯算法、数据结构契约、组件状态机 |
| Harness 集成测试 | `agent/test/harness/compaction.test.ts` | `tests/bdd.rs` + `tests/features/` | rstest-bdd | 端到端编排、CLI 行为 |
| E2E provider | `ai/test/context-overflow.test.ts` | 暂按需 | 单独 crate/CI | 真实 provider API 验证 |

### 分界规则

```
BDD 独占（不同时写单元测试）：
  └─ agent 循环 turn 流程（prompt → model → tool → result → loop）
  └─ 工具执行完整路径（参数验证 → 执行 → 输出格式化）
  └─ session 持久化读写
  └─ CLI 命令分发与错误输出
  └─ hooks 执行序列

单元测试独占（不同时写 BDD）：
  └─ 纯数据结构序列化/反序列化
  └─ 无副作用的算法（cut point、token estimate）
  └─ 组件内部状态机（queue、retry）
  └─ 字符串解析（命令、模板）
  └─ 错误类型 Display/From
  └─ 类型枚举变体匹配

重叠区域（两者都可，但应有不同角度）：
  └─ 如果已在 BDD 验证了 "agent 正确处理工具调用"，
      单元测试只测该路径用到的底层函数边界（如参数 schema 构造、结果格式化），
      不重复验证全流程
```

## 测试组织约定

1. 每个 `core/` 子模块必须有一个 `#[cfg(test)] mod tests`。
2. 每个 `agent/` 纯逻辑文件（queue、retry、commands、templates、config_value）优先在文件内编写测试。
3. Session 子组件（model_manager、tool_manager、skill_manager）的测试放在各自文件内。
4. 已有 BDD 场景覆盖的模块（agent loop、tools、session 持久化）不重复写 BDD 级别的单元测试。
5. 测试应使用 `$CARGO_MANIFEST_DIR` 引用 fixture 文件（如需要），避免 CWD 依赖。

## 快速校验命令

```bash
# 增量检查新测试
cargo test --lib agent::queue
cargo test --lib agent::retry
cargo test --lib agent::commands
cargo test --lib core::error
cargo test --lib core::model
cargo test --lib core::types
cargo test --lib core::traits

# 全量回归
cargo test --lib
cargo test --test bdd
```
