# Design: c1890 ContextPolicy + ResponsesAssembler

## 目标

在已归档 `c1880`（`WirePolicy` / `compat` / `extra_policy`）之上，引入 **ContextPolicy + ResponsesAssembler** 缝：同一 session 投影 → 唯一 Responses body 构造点；默认行为 **字节级等价现状**；为状态栏 / tool_search / 日界 / epoch 留钩子，本 change **不**实现它们。

```text
AgentMessage[]  --project_for_llm-->  AiBridgeMessage[]
                                              │
SystemPromptOpts / tools schemas              │
                                              ▼
                                   ContextPolicy::default()
                                              │
                    WirePolicy::default() ────┤
                                              ▼
                                   ResponsesAssembler
                                              │
                                              ▼
                                   /v1/responses JSON body
```

## 书 / research 边界

- 书（`ai-agent-book/book/chapter2.md`）提供「静态前缀 + 轨迹」「KV/Prompt Cache 前缀敏感」直觉。
- 工程 SSOT = 本 design + live specs（Specs landing 后）；书语不进 requirement 正文。
- 术语：`flavor`（旧稿）→ **`compat`**；散落 capabilities（旧稿）→ wire 侧 **`extra_policy`**（agent 能力另案）。

## 落点（层）

| 组件 | 建议层 | 说明 |
|---|---|---|
| `ContextPolicy` | `agent/`（或 `protocol/` 若需跨面共享只读视图） | 策略档；默认板 `defaults.rs` |
| `ResponsesAssembler` | `package-ai-bridge`（靠近 `assemble_responses_body`） | 唯一 body 构造；读 `WirePolicy` |
| 调用缝 | `agent` ReAct / infra provider map | 经 Assembler；adapter **不**二次重排业务布局 |

禁止：`agent` → 直接拼 Responses 字段；`infra` 发明第二套 layout。

## 默认等价现状（验收锚）

今日路径（须 golden 对照）：

1. `prompt::build_system_prompt` → `Session.system_prompt`
2. `AiBridgeGenerateOptions.system_prompt` → `prepend_system_prompt_item`（thinking on → `developer`，off → `system`）
3. `assemble_responses_body` + `apply_responses_wire_policy`
4. tools = 当前全量 schema（`tools_mode=full`）

Assembler 默认档 MUST 产出与上述等价的 body（允许键序/空白差异由规范化比较）。

## ContextPolicy 形状（意向）

```text
ContextPolicy {
  tools_mode: Full | Search,          // Search 行为 → c1900
  status_bar_mode: Off | Replace | Append,  // → c1895；默认 Off
  allow_midturn_tools_rewrite: bool,  // Full 默认可 true；Search 默认 false
  date_placement: SystemAsToday,      // 占位枚举；默认 = 现状
  // 预留：day_boundary / epoch 读取点（c1920）
}
```

`ContextPolicy::default()` 只组合 `agent`（或 bridge）侧 `defaults.rs` 常量；**无** YAML / env。

## WirePolicy 消费

- Assembler 构造时注入 `WirePolicy`（默认 `WirePolicy::default()`）。
- `compat == Generic`：保守字段子集（与 `c1880` / 现 `apply_responses_wire_policy` 一致）。
- `extra_policy`：本 change **不**新开位；仅透传既有闸门（如 `prompt_cache_key` 仍 false → 不写键）。
- 测试用结构体字面量覆盖，证明「同投影、不同 WirePolicy → 字段集 diff」。

## `instructions` 决策（已钉）

**不**写顶栏 `instructions` 第二份；继续 `system_prompt` → input 前缀 item。避免双份 SSOT。

## 测试缝（seam）

| 缝 | 方式 |
|---|---|
| `ResponsesAssembler::assemble(...)` | 包内 golden：固定投影 + policy + WirePolicy → body JSON（规范化后稳定） |
| `ContextPolicy::default()` | 单测：钩子默认值 |
| ReAct / provider 调用点 | 静态断言或薄集成：Responses 路径经 Assembler（`feature: false` 场景） |
| 默认 ≡ 现状 | 对照 fixture：Assembler(default) vs 旧 `assemble_responses_body` 路径 |

**不**扩 BDD step（与 `c1880`/`c1885` 一致）；合约场景 `feature: false`。

## 非目标

- StatusBar / tool_search / compaction / previous_response_id / epoch 实现
- Completions 布局重写；Anthropic Assembler
- 自动探测网关；命中率 KPI
