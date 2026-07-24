---
change_id: c1580-fix-tui-busy-slash-not-steer
title: busy 已识别 slash 经 BusySlashPolicy 分流；MUST NOT 落入 steer
status: ready
priority: 1580
depends_on: []
author: agent
branch: feat/c1580-busy-slash-not-steer
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1580-fix-tui-busy-slash-not-steer

> **决策已全部锁定**（含 pi 调研）。延后修复；高质量实现要求见 D2。

## Why

busy 中 `/session-name` 等落入 Steering（图 1–2）。pi：已识别 builtin/extension slash **先于** streaming 分支执行，**永不**当 steer；仅 `/reload` 硬拒绝。

## 代码事实

- xylitol：`try_busy_input` 只特判 reload/trust/theme/history-copy；其余 `PendingSlash` 落入 `_` → steer
- pi：无 `availableDuringStreaming` flag；靠 onSubmit 先拦截 + `prompt` 内 extension 优先。xylitol 已有 `PendingSlash` 枚举 → **适合声明式 policy**，优于复制 pi 巨型 if-chain

调研摘要：[pi busy slash](fb11a298-0769-42b9-a2b8-8a4b46cac721)

## Decisions（已锁）

### D1. 总闸

`parse_slash_command` 命中（含 `Usage`）→ **MUST NOT** steer / follow-up。
未识别且以 `/` 开头 → **MUST NOT** steer；idle 风格短提示（比 pi「未知当 steer」更严，避免污染 context）。

### D2. 实现形状（高质量）

- 在 `commands`（或邻接）提供 **`BusySlashPolicy::{Allow, Reject}`**（或等价 trait/`fn policy(PendingSlash) -> …`）
- **单表 / 单 match 穷尽** `PendingSlash`；`try_busy_input` **只**查表，禁止再散落 if
- Allow → 清 editor → `pending.slash` / 既有 effects（与 idle 同源泵）
- Reject → 清或保留 editor（与现 reload 一致：清 + system note），**不**入队
- 新命令 **MUST** 进表（编译期穷尽 match 防漏）

### D3. 分类表（对齐 pi；保留 xylitol 有意更严项）

| Policy | `PendingSlash` | 说明 |
|---|---|---|
| **Allow** | `SessionName`、`SetModel`、`HistoryCopyLast`、`SessionDump`、`Export`、`Exit`、`Compact` | 对齐 pi `/name` `/model` `/copy` `/session` `/export` `/quit` `/compact`（compact 可先 abort 再压实，与 pi 一致） |
| **Reject** | `Reload`、`Trust`、`Theme` | reload=pi；trust/theme=xylitol 更严（PI_DELTA，保持） |
| **Reject** | `OpenModels`、`OpenTree`、`ForkAtLeaf`、`OpenSessionResume`、`SessionNew`、`SessionClone`、`Import`、`DebugScene` | 开槽 / 切会话 UI：沿用现有 idle-only 合约（`ati21`/`ati28` 等）；**仍禁止 steer** |
| **Reject** | `Usage` | 用法提示，不入队 |

有参 `/model <id>` = `SetModel` → **Allow**（NextTurn）；无参开列表 = `OpenModels` → **Reject**（与 ati21 一致，可另 change 再对齐 pi 开 picker）。

## Open Questions

（已清空。）

## Non-Goals

- c1585 drain 模式；插件市场；第二套 slash 泵

## Related

- `app-tui-input`、`app-tui-commands`；architecture「禁止 slash 当 steer」

## Ethics

- `ethics.risk_level`: low
- `ethics.required_evidence`: busy `/session-name` Allow + `/reload` Reject + 未知 `/foo` 不入 steer harness
- `ethics.escalation_policy`: 无
