---
change_id: c735-update-agent-hooks-pi-parity
title: "Track A — pi hook 对齐调研矩阵与分波计划"
status: purpose-draft
---

# Track A 调研：pi hooks ↔ xylitol

SSOT 对照源：

- pi：`../pi/packages/coding-agent/docs/extensions.md`（Lifecycle Overview + Events）
- pi AI 缝：`packages/ai` `onPayload` / `onResponse`
- xylitol 脚本：`src/infra/hooks/`（`HookEvent` + `HookDispatcher`）
- xylitol 热路径：`src/agent/runtime/hooks.rs` `AgentHooks` ← `react.rs`
- 合约：`llmanspec/specs/agent-hooks/spec.toon`（**部分超前于实现**）

抓包 / MITM / claude-tap：**不在本文件** → `c999-add-infra-provider-traffic-capture`。

---

## 1. 架构双轨（对齐前必须钉死）

| 层 | pi | xylitol 现状 | 目标 |
|----|-----|--------------|------|
| 用户扩展 | `pi.on("…")` 扩展脚本/TS | 配置驱动 **脚本** `HookDispatcher` | 保留；事件名对齐 pi |
| 进程内回调 | harness / AI `onPayload` | **`AgentHooks`**（before/after tool、transform_context） | 保留；provider 缝补进 **adapter**，再桥到脚本 |
| 传输 | `onPayload` / `onResponse`（headers only） | **无** | Wave 0 必做 |
| 流式 token | `message_update` 等 | `XyEvent` 流（非 hook） | 可选桥接 hook；非抓包 |

**发现**：`HookDispatcher` 今日主要被 BDD/单测驱动；**ReAct 不调用** `dispatch(HookEvent::…)`。产品热路径只有 `AgentHooks`。

**发现**：spec h7 写作 `after_provider_request`，pi 为 **`before_provider_request`**（可改 payload）。h8 语义接近 pi `after_provider_response`，但 **未实现**。

---

## 2. 完整对照矩阵

图例：

- **P** = pi 有
- **X-live** = xylitol 热路径已接线
- **X-enum** = 仅有 `HookEvent` / 类型，未进 ReAct
- **X-spec** = 仅有 spec/BDD（可能空 stub）
- **—** = 无

| # | pi 事件 | 可看/可改 | xylitol | 状态 | 分波 |
|---|---------|-----------|---------|------|------|
| 1 | `project_trust` | 信任决策 | `infra/trust` + TUI trust gate（**非 hook**） | 语义有、非 extension hook | W4 可选桥 |
| 2 | `resources_discover` | skill/prompt/theme 路径 | skills/MCP 装配路径不同 | 缺失（产品模型不同） | W5 或 skip |
| 3 | `session_start` | reason / previous file | `SessionSnapshot/Spawn/Merge` 不同模型 | 部分 / 不等价 | W3 |
| 4 | `session_info_changed` | name | — | 缺失 | W3 |
| 5 | `session_before_switch` | cancel | — | 缺失 | W3 |
| 6 | `session_before_fork` | cancel | Driver fork；无 hook | 缺失 | W3 |
| 7 | `session_before_compact` | cancel / custom | compaction 在 agent；无 hook | 缺失 | W3 |
| 8 | `session_compact` | 观察 | — | 缺失 | W3 |
| 9 | `session_before_tree` / `session_tree` | cancel | session-tree 产品面；无 hook | 缺失 | W3 |
| 10 | `session_shutdown` | 清理 | — | 缺失 | W3 |
| 11 | `before_agent_start` | 改 system / 注入 | — | 缺失 | W2 |
| 12 | `agent_start` / `agent_end` | 观察 | `XyEvent::AgentStart/End`；无脚本 hook | 部分 | W2 |
| 13 | `agent_settled` | 无后续 | — | 缺失 | W2 |
| 14 | `turn_start` / `turn_end` | 观察 | `XyEvent::Turn*`；spec 提 turn_end 未进 enum | 部分 | W2 |
| 15 | `message_start` / `update` / `end` | 观察 | `XyEvent`；spec message_end 未进 enum | 部分 | W2 |
| 16 | `tool_execution_start/update/end` | 观察 | `XyEvent::ToolExecution*` | 部分（非脚本） | W1 |
| 17 | `context` | 改 messages | **`AgentHooks.transform_context`** | **对齐（进程内）** | W1 桥脚本 |
| 18 | `before_provider_headers` | 改 headers | — | **缺失** | **W0** |
| 19 | `before_provider_request` | 改/替换 body | spec 错名 `after_provider_request`；BDD 空 | **缺失** | **W0** |
| 20 | `after_provider_response` | status+headers；**无 body** | spec h8；BDD 空 | **缺失** | **W0** |
| 21 | `model_select` | 观察/干预 | `ModelQuery` 仅 model+prompt_length | 弱部分 | W2 |
| 22 | `thinking_level_select` | 观察 | ThinkingLevel 设置路径；无 hook | 缺失 | W2 |
| 23 | `tool_call` | block | **`AgentHooks.before_tool_call`** + enum `ToolCall` | **部分**（脚本未进热路径） | W1 |
| 24 | `tool_result` | modify | **`AgentHooks.after_tool_call`** | **部分** | W1 |
| 25 | `user_bash` | bang 相关 | TUI bang；无 hook | 缺失 | W4 |
| 26 | `input` | 拦截用户输入 | TUI host；无 hook | 缺失 | W4 或 skip |
| — | — | — | `PlanGenerated` / `Review*` / `RepeatDetected` / `StepRetry` / `ToolCallBlocked` / LSP/DAP/file_write | **xylitol-only** | 保留；文档化 |
| — | raw SSE hook | — | — | **双方都无** | → c999 |

### Provider 可见性（与抓包边界）

| 阶段 | pi | xylitol 目标 |
|------|-----|--------------|
| 请求 headers | `before_provider_headers` | 同 |
| 请求 body | `before_provider_request` | 同（prefix-cache） |
| 响应 headers | `after_provider_response` | 同 |
| 成功 SSE body | **不进 hook** | **不进 hook**（c999 / DUMP） |
| 已解析 delta | `message_update` | `XyEvent::*Delta`；可选 post hook |

---

## 3. 接线证据（实现前基线）

| 断言 | 证据 |
|------|------|
| Provider hook 未实现 | `HookEvent` 无 provider 变体；`tests/bdd.rs` 中 `test_hook_provider_*` 空 body |
| 脚本未进 ReAct | `react.rs` 只读 `AgentHooks`；无 `HookDispatcher::dispatch` |
| Trust 非 hook | `infra/trust` + `trust_gate` / bootstrap |
| Spec 超前 | `agent-hooks` h1/h7/h8 列出 after_provider_* |

---

## 4. 分波实现计划（逐一完成）

> 优先级数字为建议；开 change 时再定正式 `cNNN` id。本 umbrella `c735` 保留为路线图；每波独立 propose/apply/archive。

### Wave 0 — Provider 三缝（阻塞假合约）建议 id：`c740-…`

1. Adapter / `XyModel` 路径增加等价于 pi 的：
   - `before_provider_headers`
   - `before_provider_request`（可替换序列化 body）
   - `after_provider_response`（status + headers，**流消费前**）
2. 修正 `agent-hooks` spec：删除/改名错误的 `after_provider_request` → `before_provider_request`。
3. `HookEvent` + `HookDispatcher` 接线；BDD 空 stub → 真测（可用 Fake adapter 记录回调）。
4. **验收**：deepseek-style `cache_control` 类 modify 场景可测；空配置零开销。

### Wave 1 — 工具 + context 脚本桥 建议 id：`c745-…`

1. ReAct：`before_tool_call` / `after_tool_call` / `transform_context` 在调用 `AgentHooks` 同时（或经统一 facade）dispatch 脚本事件。
2. 对齐命名：`tool_call` / `tool_result` / `context`（或保留 phase 模式 `pre.tool_call` 并文档对照 pi）。
3. 可选：从 `XyEvent::ToolExecution*` 派生 `tool_execution_*` 脚本通知（只观察）。

### Wave 2 — Agent / turn / message / model 建议 id：`c750-…`

1. 在现有 `XyEvent` 发射点旁挂脚本 hook：`agent_*`、`turn_*`、`message_*`。
2. `model_select` / `thinking_level_select` 接到 Driver/settings 变更。
3. `before_agent_start` / `agent_settled` 补语义（settled = 无 follow-up/retry/compaction 排队）。

### Wave 3 — Session 树 / compact / fork / switch 建议 id：`c755-…`

1. 映射 xylitol 已有 Driver/session-tree/compaction 操作到 pi 同名 cancel/observe 钩子。
2. 不强行复制 pi `SessionManager` 文件模型；事件 payload 用本仓 id/path。

### Wave 4 — Trust / bang / input（可选）建议 id：`c760-…`

1. `project_trust`：可选让脚本参与（今日 trust 已够用则可 **skip** 并文档说明）。
2. `user_bash`：TUI bang 边界。
3. `input`：仅当有明确扩展需求（易与 TUI host 抢键）。

### Wave 5 — resources_discover / UI API（低优先或 skip）

1. pi 的 skill/theme 发现 vs xylitol skills/MCP —— 产品不等价则 **正式 skip** 并写进 AGENTS。
2. `ctx.ui` / custom tool render —— 单列大 change，不堵 W0–W3。

### xylitol-only 清理波（可穿插）建议 id：`c765-…`

1. 文档化 `PlanGenerated` / `Review*` / `RepeatDetected` / … 哪些 live、哪些死。
2. 死事件：删或标注 reserved；避免 BDD 假绿。

---

## 5. 命名与 API 约定（实现对齐用）

| 规则 | 说明 |
|------|------|
| 对外事件名 | **优先 pi 字符串**（`before_provider_request`） |
| 配置匹配 | 可继续 `pre.`/`post.` phase 模式，但 provider 三缝用 pi 全名做别名 |
| 双轨 | `AgentHooks`（Rust 闭包，低延迟）+ 脚本 dispatcher（IO）；同一逻辑事件两边都可订 |
| Fail-open | 脚本超时 = allow（已有 h5） |
| 密钥 | hook stdin 默认脱敏 Authorization；完整 body 仅 debug |

---

## 6. 与 Track B（c999）边界

```
W0 before_provider_request ──► 可改 JSON body（产品）
c999 mitm/claude-tap/dump ──► 看原始 SSE（诊断）
DUMP_CHUNK（可选）──────────► 看 ThinkingDelta vs TextDelta（映射层）
```

三者互补；**禁止**用 hook 替代抓包，也 **禁止** 等抓包完成再开 W0。

---

## 7. 建议验收总闸（全波完成后）

1. `llmanspec/specs/agent-hooks` 与代码一致（无空 stub MUST）。
2. 对照表 §2 中 W0–W3「缺失」清零或显式 skip 有记录。
3. `just qa` 相关 BDD 绿。
4. 示例脚本：prefix-cache modify + 打 status/headers 日志（对齐 pi `provider-payload` 示例精神）。

---

## 8. 立即下一步

1. 用户确认分波与 skip 项。
2. 开 **Wave 0** 正式 propose（full）：provider 三缝 + 修 h7 命名。
3. 并行：`c999` B1/B2 PoC（不挡 W0）。
