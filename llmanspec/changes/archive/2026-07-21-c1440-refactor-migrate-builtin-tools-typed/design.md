# Design: c1440-refactor-migrate-builtin-tools-typed

## Approach

机械迁移：每个内置工具把 `impl XyTool` 换成 `impl TypedTool`，逻辑原样搬进 `execute_typed` / `execute_as_parts_typed`；`parse_tool_args` 只留在 `typed.rs` blanket。

## Decisions

| 决策 | 选择 | 理由 |
|---|---|---|
| 对外口 | 不变 `dyn XyTool` | MCP/组合根/测试仍走 Value 入口 |
| Args 可见性 | `*Args`（及嵌套 Deserialize 类型）`pub` | `TypedTool::Args` 关联类型要求 |
| `read` | 核心在 `execute_as_parts_typed`；`execute_typed` 委托 parts→string | 保持图文 parts 与现 execute 折叠语义 |
| `edit` Sequential / guidelines | 在 TypedTool 覆写同名钩子 | 与现 XyTool 覆写等价 |
| live specs | 不改 | 纯内部重构；行为字面保持 |

## Risks

- 迁移时误改错误文案 / 输出形状 → 单测 + `just qa` 抓住；若必须改文案才能绿则 STOP。
