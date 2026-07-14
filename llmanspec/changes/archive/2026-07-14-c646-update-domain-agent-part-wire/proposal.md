---
change_id: c646-update-domain-agent-part-wire
title: "AgentPart JSONL wire 对齐 pi（tagged；无旧格式兼容）"
status: full
priority: 646
depends_on: []
author: agent
track: A
---

# c646-update-domain-agent-part-wire

## Why

会话持久化已写入 thinking，但 `AgentPart` 使用 `#[serde(untagged)]`：Thinking 无 `type`、字段叫 `text`，Text 为裸字符串。travel / fork / 冷启动用 `message_text()` 把部件压成单一 `Assistant`，直播时的可折叠 Thinking **不可幂等恢复**。这是后续树、fork、resume、导出的偏移根基，必须先钉死 wire。

对照 `../pi`：`{type:"thinking",thinking,...}` / `{type:"text",text}` / `{type:"toolCall",...}`；UI 按 `content[].type` 重建。

## Purpose

1. **唯一真源**：`AgentPart` + session entry 外壳均为 **tagged / camelCase** wire（对齐 pi 实盘；决议 1+2）。
2. **无兼容**：不读不迁旧 untagged / snake 外壳；上线前清 `~/.xylitol/sessions`；非法 content 按 E1 跳过/失败可观测。
3. **可恢复**：persist → load → 部件结构不变；TUI rebuild ≡ 直播分块。
4. **预填纯净**：`message_text` 只聚合 `type=text`。
5. **可扩展底座**：domain 自有 tag union；日后可脱离 pi 生态自行加 variant / 升 v6。

## What Changes

1. `SESSION_VERSION = 5`；`EntryBase`/`SessionHeader`/`SessionEntry` type 名与字段 camelCase（`parentId`、`parentSession`、…）。
2. `AgentPart`：`#[serde(tag = "type")]`；`thinking`/`thinkingSignature`；禁止裸 Text；禁止 content 内嵌 toolResult（G2）；Image `mimeType`。
3. `AgentMessage` toolResult：`toolCallId`（C1）。
4. `session_entry_to_ui*` 按 parts 投影 Thinking+Assistant；travel/fork 共用。
5. 全仓 fixture/snapshot 更新；闸全绿。

## Capabilities

- `domain-message`（新：AgentPart wire SSOT）
- `agent-session`（persist / as_agent_message）
- `agent-session-store`（修改 s4：不迁移旧 AgentPart）
- `app-tui-transcript`（历史重建分块）

## Design SSOT

- 本 change `design.md`（pi 对拍表 + 拒绝策略 + 实现顺序）

## Impact

- `src/domain/message.rs`、`session_types.rs`（`message_text`）
- `src/agent/compaction/message_converter.rs`、`runtime/react.rs`（persist 路径）
- `src/app/tui/bridge/session_tree.rs`（及调用方 travel/fork）
- 所有依赖旧 content fixture 的单测 / snapshot / BDD
- 用户侧：既有 `~/.xylitol/sessions/*.jsonl` 旧 content **不可再当合法对话恢复**（需新开会话）

## Out of scope

- 未知 `type` 前向兼容读取（`Unknown` variant）— future
- 树动态 hint（L3）、真模型 qwen E2E（L4）
- c645 fork 键位本身（rebuild 真源在本 change）
- 旧 JSONL 迁移工具（清库即可）

## Ethics

- risk_level: high（破坏性：旧会话不可读）
- prohibited_actions: 静默把旧 untagged 当 Thinking；只改 TUI 猜测而不改 wire；保留 untagged「双路径」冒充兼容
- required_evidence: tagged round-trip 单测；legacy 拒绝测；rebuild Thinking+Assistant harness；design 对拍表
- escalation_policy: 若 provider 回放需要 signature 字段名与 pi 不一致，先停并对照 `../pi/packages/ai/src/types.ts` 再定

## Depends

- 无（不依赖未归档的 c645；可与 c645 并行，但 **apply 本 change 应优先于**依赖正确 rebuild 的 fork UX 验收）
