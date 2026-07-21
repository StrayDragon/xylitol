---
change_id: c1220-refactor-fuse-domain-runtime-protocol
title: 消除纯 domain 层——类型就地融合；ports 收拢；agent ≈ pi-agent-core
status: purpose-draft
priority: 1220
depends_on:
  - c1210-refactor-compose-bridge-llm
author: agent
---

# c1220-refactor-fuse-domain-runtime-protocol

> **状态**：purpose-draft（仅意向说明，未 attach / 未改 live specs）。
> **依赖**：MUST 先完成并归档 **c1210**（组合 bridge LLM DTO + bash 嵌 message）。
> **命名**：今日 `src/protocol/`（线协议）与 `src/runtime_protocol/`（ports）本是同一类「跨边界契约」被拆成两个顶栏；本 change 意向是 **收拢为一致的 `protocol/`（可分子模组）**，并消掉纯 `domain/`——不是永久维护两套「protocol」品牌。

## Why

今日分层把「共享纯类型」堆在 `src/domain/`，把「ports」堆在 `src/runtime_protocol/`，与 pi 的 **agent-core（类型与循环同住）← pi-ai（LLM 叶）** 心智不一致：

- `XyModel::generate_stream(Vec<AgentMessage>)` 让会话词汇穿过适配边界，才被迫维持独立 domain 桶。
- pi 中 `convertToLlm` 在 agent-core 内完成，LLM 侧只见 `Message[]`。
- 用户意向：`src/agent` ≈ pi-agent-core；**消掉纯类型层**；能就地融合的放回 agent / session / config 等；ports 与少量跨面契约收拢到更清晰的「通用」位置。

## 与 c1210 的顺序（决议）

| 先做 | 原因 |
|---|---|
| **先 c1210，后本 change** | c1210 钉死「LLM 叶 SSOT = bridge、AgentMessage 组合之、bash 落盘同构」。大重构若先做，会在孪生叶 + 顶层 bash 上搬两次。 |
| 本 change **depends_on c1210** | 组合与投影语义稳定后再搬目录/改端口。 |

**不要**并行大开：c1210 apply 期间只动消息/投影/session bash；本 draft 只文档占位。

## 目标形状（意向，非正式合约）

```text
packages/xylitol-ai-bridge     ≈ pi-ai（LLM 叶 DTO + adapters）
        ↑
src/agent/                    ≈ pi-agent-core
  AgentMessage = Llm(bridge) | Env
  project_for_llm / ReAct / compaction / prompt / session façade
        ↑
src/protocol/                 跨边界契约（线协议 Command/Event + ports XyModel/…）
  XyModel 入参 = Vec<AiBridgeMessage> —— 不再吃 AgentMessage
        ↑
src/infra/                    实现：adapter、JSONL IO、tools…
src/app/                      应用面
```

消掉 **`src/domain/` 作为独立「纯类型层」**；不是把所有类型塞进一个新超级目录。

## 命名与收拢（修订）

今日两套顶栏都叫「protocol」，却分家：

| 今日 | 实际装的 |
|---|---|
| `src/protocol/` | client↔core `Command` / `Event`（线协议） |
| `src/runtime_protocol/` | agent↔infra ports（`XyModel` / `XyTool` / …） |

**合理实践**：这是**同一类东西**（跨边界契约），拆成两个顶层目录多半是过度分层，不是必要隔离。目标形态倾向：

```text
src/protocol/
  command.rs / event.rs / transport.rs   ← 线协议（已有）
  model.rs / tool.rs / session.rs / …    ← 今日 runtime_protocol ports
  （可选）少量真正跨面共享的枚举/DTO —— 仅当多处签名都要用
```

子模组可以按稳定性分开（wire vs ports），但 **一个 `protocol/` 品牌即可**。
仍须避免的是：把 `AgentMessage` / session JSONL 词汇无差别地倒进 `protocol/` 根上与 `Command` 糊成一锅——那些应 **就地进 `agent/`**（≈ pi-agent-core），不是再造纯类型桶。

「有的直接融合回合适的地方」优先于「换皮保留 domain」。

## 类型落点草图（正式化时再写成 tasks）

| 今日 | 意向归属 |
|---|---|
| `message` / `llm_project` / Env | **`agent/`**（与 loop 同住） |
| `XyModel` 消息入参 | **LLM DTO only**；投影只在 agent |
| `session_types` | agent/session 与/或 infra/session **与实现同住**；避免第三份 SSOT |
| `XyEvent` | 靠 `XyEventSink` / 线协议再导出；不单独 domain |
| `types`（Chunk/ToolSchema/thinking） | port 签名旁或 bridge / config 按消费方 |
| `model` Kind/Config | agent/model 或 infra config |
| `error` / resource / source_info | 跟 port 或 loader |
| `tool_result_quiet` / `text` | **agent/** |
| `runtime_protocol/*` traits | **并入 `src/protocol/`**（与 Command/Event 同顶栏；可分子模组） |

## What Changes（正式 propose 时）

- 改写 `layer-architecture` / `domain-message` / provider 相关 live specs（la1/la2/dm6/pa20…）
- 搬模块、改 `XyModel` 签名、删 `src/domain/`
- 更新 `src/AGENTS.md` 分层图与 `pub use`
- 全量编译 + BDD + `just qa`

## Out of scope（本 draft）

- 本 change **不实现**；不改 live specs
- 不抽多 crate；不改 Trust/MCP 产品语义
- 不重做 `src/protocol/` 线协议（除非显式另开 change）

## Capabilities（预期触及）

- `layer-architecture`、`domain-message`（或后继改名）、`infra-provider`、`agent-runtime`、`agent-session`、`package-ai-bridge`（边界措辞）

## Ethics

- risk_level: critical（全仓分层与端口签名）
- prohibited_actions: 在 c1210 未归档前开写本 change 实现；为「纯度」再造第三个纯类型顶栏；把 AgentMessage 与 Command 无结构地混在同一文件级 SSOT
- required_evidence（正式化后）: `XyModel` 不再接受 `AgentMessage`；`src/domain/` 与 `src/runtime_protocol/` 删除或仅迁移期 re-export；契约集中在 `src/protocol/`；agent 不依赖 infra；infra 不依赖 agent
- escalation_policy: 正式 propose 前确认 session 类型最终在 agent vs protocol 子模组的边界

## 建议下一步

1. **Apply c1210**（当前分支已 propose）。
2. c1210 archive / merge 后，再对本 id **正式 propose**（补 design/tasks、改 live specs、attach）。
3. 正式化时默认：**ports 并入已有 `src/protocol/`**；类型能进 agent 的不进 protocol。
