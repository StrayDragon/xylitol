---
change_id: c1690-fix-session-jsonl-camelcase-ssot
title: 冻结 Session JSONL camelCase SSOT 并剔除旧栈
status: designed
priority: 1690
depends_on: []
author: agent
branch: sdd/c1690-fix-session-jsonl-camelcase-ssot
base_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
checkpointed: true
checkpoint_sha: 8d1e8ba37a96b2a4d0d8375c071628224dcedddc
---

# c1690-fix-session-jsonl-camelcase-ssot

> **产品拍板**：Session persisted JSONL **回归并钉死 camelCase**（JS/TS favor）；不追 pi 盘格式；v0.0.0 破坏性只维护最新。
> **姊妹提案**：compaction OTel 挂载见 `c1700-add-otel-compaction-spans`（独立，不依赖本 change）。

## Why

盘上 JSONL 已是 camel entry shell / type tags，但合约与实现仍残留：

- `s2` 仍写 snake 类型名（`branch_summary`…）与真源 `s18` 冲突
- `SESSION_VERSION=5` 但 `create` 常写 `version: 4`
- 读路径仍有 `bash_execution` alias、顶层 bash lift、v3→v4 migrate
- 坏行/未知 type 可拖垮 `list_sessions` / resume

需要 **单一 SSOT + 闸测**：新写只出最新 camel；旧栈直接 skip，warn≤3 后 `...`，保护 TUI / print / CLI。

## What Changes

- **SSOT**：Session JSONL 外壳字段与 `type` 判别 = **camelCase**（`parentId`、`branchSummary`、`thinkingLevelChange`、`sessionInfo`…）；嵌套 message role/part tag 维持 camel（`bashExecution`、`toolCall`…）。
- **新写**：`header.version` MUST = `SESSION_VERSION`（5）；bang-bash **仅** `type=message` + `role=bashExecution`；MUST NOT 写顶层 `bashExecution` / `bash_execution`、`parent_id`、version≤4。
- **读旧栈**：未知 `type`、非 SSOT snake tag、无法解析行、旧 untagged content → **跳过该行**；进程内 warn **至多 3 条**，其后合并为 `...`；MUST NOT serde `alias`、MUST NOT 静默 migrate/lift 当合法上下文。
- **面保护**：`load` / `list_sessions` / TUI resume / print `--session` 同源策略；单文件坏行 MUST NOT 使整表 list 失败。
- **刻意不改**：wire `Command`/`Event` 保持 snake；LLM bridge 出站投影不变。

## Capabilities

`agent-session-store`（主）· `agent-session`（as45/as46）· `infra-bash`（be4/be6 去掉读提升）

## Impact

- 本机旧 JSONL（snake `bash_execution`、`parent_id`、v3）恢复时对应行丢失，有 warn；符合 v0.0.0 只维护最新。
- 未来 TS web/app 可直接消费盘文件，少一层 rename。

## Ethics

- risk_level: medium（破坏性读路径）
- prohibited_actions: 扩 serde alias；静默 v3/v4 migrate；为兼容 pi 改写盘格式；把 wire Command 改成 camel
- required_evidence: 黄金样例 round-trip；拒绝样例 skip+warn≤3；create version=5；list 遇坏文件不崩
- refusal_contract: 不做「读时改写盘文件」的自动升级工具（除非用户另开 change）
- escalation_policy: 若 print/TUI 对 warn 展示不一致，可 follow-up 对齐文案，不阻塞本 SSOT
