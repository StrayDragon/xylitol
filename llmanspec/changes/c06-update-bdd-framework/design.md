# 设计文档：BDD 框架迁移

## 迁移策略

### 方式：全量重写 vs 增量迁移

选择 **全量重写单文件 `tests/bdd.rs`**，原因：
- 只有一个 BDD runner 文件（1191 行），没有分模块
- Feature 文件（12 个）不改动
- 迁移过程可以用 `git stash`/分支隔离，完成后一次切换

### Fixture 拆分方案

当前 `ToolWorld` 有约 20 个字段。按功能域拆分为 3 个独立 fixture：

| Fixture | 字段 | 说明 |
|---------|------|------|
| `Workspace` | `workspace: TempDir` | 文件系统夹具，所有工具测试共用 |
| `SessionStore` | `session_mgr, session_entries, current_session_id` | 会话测试 |
| `AgentState` | `model_registry, agent_events, last_result, ...` | Agent 循环测试 |

### 步骤模式迁移方式

所有 regex 到 typed placeholder 的映射已有对照表（见 `_MIGRATE_SUGG.md`）。重复模式提取为常量或宏以避免重复。

### 场景绑定策略

为每个 feature 文件创建一个测试模块文件，每个场景一个 `#[scenario]` + `#[test]` 函数。
聚合到一个入口模块 `tests/bdd.rs`（或改为 `tests/bdd/mod.rs`）。

但为最小化 diff，保持单文件结构：`tests/bdd.rs` 中包含所有 step 定义 + 场景绑定。

### 数据表格与 Doc String

- Data Tables：现有使用 DataTable 的场景（edit.feature 的多处替换 / hooks.feature）改为 `#[datatable]`
- Doc Strings：`Background` 的多行内容改为 `docstring: String` 参数

### 风险

1. **中文列名** — 当前 feature 文件未使用 Scenario Outline 中文列名（仅场景名中文），无风险
2. **Rule 内场景** — 当前无 Rule 块，无风险
3. **gherkin 解析器不一致** — rstest-bdd 使用 gherkin 0.14+，cucumber 0.23 使用 gherkin 0.13+，两者解析行为可能略有差异。遇到解析失败时简化 Examples 表
4. **`&mut` + placeholder 冲突** — 所有步骤函数签名使用 `&State`（不可变引用）+ `Cell`/`RefCell` 内部可变性
