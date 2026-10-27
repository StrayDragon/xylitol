# Design: c2950

## 1. 为什么 skills 展开是内置项，不是可注册插件

三条约束同时成立：

| 约束 | 来源 |
|---|---|
| 未配置第三方中间件时 root 链须为空，空链不得产生 span | `r2931`（c2946 落地） |
| 未配置时不得注册任何中间件 | `r2925` |
| skills 展开是**必需**行为（有 skills 就展开，无 skills 就 no-op） | `skill_expand.rs:87` |

若把展开做成「默认启用的可注册插件」，前两条直接被打破：root 链不再为空。

而它又不能像 loop-example 那样「配置才启用」——skills 展开不是可选功能。

因此定位为**内置项**：与 `before_model` 同阶段、先于第三方执行，但**不进入** `LoopMiddlewareRegistry`、不产生 span、不占 order 位。

## 2. 执行顺序

```text
历史快照 messages = history.clone()
→ [内置] skills 展开（仅作用于 UserMessage 的 Text part）
→ [第三方] before_model 中间件（可看到展开后的 messages）
→ 模型调用
```

关键变化：内置展开**移到** `before_model` 之前（今日是 `before_model` 在展开之后）。这让第三方第一次能观察到展开结果。

## 3. 基线等价

未注册第三方中间件时：

- `messages` 形状与今日逐字节一致（`expand_skills_in_agent_messages` 逻辑不变，只是调用点前移）；
- `chain_active == false` 时仍走原路径，不新增任何分支；
- 无额外事件、无 span。

BDD 场景：同一 skills 输入，对比「无第三方中间件」与「有第三方中间件但只读不改」两种情形下的 messages 一致。

## 4. MCP 结果截断

今日：`McpToolAdapter::execute` → `serde_json::to_string(&result)` 全量返回。

新增：官方中间件侧 `after_tool` 截断。

| 项 | 取值 | 理由 |
|---|---|---|
| 上限 | `DEFAULT_MAX_BYTES = 50 KiB`（对齐既有工具截断） | 与 read/grep/find 同一口径，用户已有心理预期 |
| 可配置 | 暂不做（Out of Scope 可后续加） | 先固定口径，避免过早抽象 |
| 截断位置 | tool result 进入 history **之前** | 保证模型看到的是截断后内容 |
| 结构化分片 | 只截断**单段 Text**；含 Image 等多分片时不动 | 遵守 `r2929` 分片保真 |

不硬编码进 MCP bridge：bridge 仍全量返回，截断由中间件承担，因此第三方可替换或关闭。

## 5. `r2931` 口径澄清

原文：「未配置中间件的 root 链 须保持为空，且空链 不得产生额外事件或观测 span。」

澄清为：「**未配置第三方可注册中间件**时 root 链须为空；loop 内置项（如 skills 展开）不计入链、不产生 span。」

这是措辞澄清，不改变 c2946 的机制（`derive()` / 会话作用域不受影响）。

## 6. 测试

| 层 | 测试 |
|---|---|
| 单测 | skills 展开前移后结果不变；空 skills no-op；第三方看到展开后的 messages |
| BDD | 基线等价（无第三方 vs 有第三方只读）；`r2931` 空链口径 |
| BDD | MCP 截断：超 50 KiB 的输出被截断且带提示；含 Image 的多分片不截断 |
| 回归 | `agent-prompt` 既有场景 |

## 7. 不做什么

不改 journal 容量、不改 `MessageUpdate` 载荷（c2952 的 Out of Scope 延续）。不让第三方替换 skills 展开。
