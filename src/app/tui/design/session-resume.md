---
version: "alpha"
name: "session-resume"
description: "/session-resume picker — editor-slot list with capped preview + full session id."
tokens_from: "../DESIGN.md"
components:
  resume-header:
    textColor: "{colors.on-surface}"
  resume-muted:
    textColor: "{colors.muted}"
  resume-selected:
    backgroundColor: "{colors.user-message-bg}"
---

# Session resume

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 键位：[`keybindings.md`](./keybindings.md) · Resume 面板。

**产品路径**：idle `/session-resume` → `EditorSlot::SessionResume`（替换 editor，非居中 overlay）。
**数据**：`XyDriver` list_sessions 缝；**MUST NOT** reach `infra::session`。

## MUST — 列表行布局

每行三列（左→右），列间固定间隙：

| 列 | 内容 | 宽度规则 |
|---|---|---|
| **预览** | `name`，否则 `first_message`；皆空则 `—`（**不**用 id 顶替预览） | **最大宽度封顶**（实现常量，约 40 列）；超长以 `…` 截断；窄终端优先压缩本列 |
| **Session id** | 完整 session id / UUID | **MUST 完整显示，禁止截断**；视口内按最长 id 对齐列宽；终端过窄时宁可整行略超宽，也不得吃掉 id |
| **元数据** | `message_count` + 相对时间（age） | 右对齐块；与既有 atm10 字段一致 |

选中行：`›` 前缀 + `{colors.user-message-bg}` 整行 wash（**MUST NOT** reverse-video）。

视口：最多约 10 行可见 + `(i/n)` 滚动提示（与 pi SessionList 对齐）。

## MUST NOT

- 把 session id 藏进仅 filter / 调试旁路；排查错误时 id 必须在列表行直接可读。
- 为塞满预览而截断 id。
- 在无 name/`first_message` 时把 id 同时画进预览列与 id 列（重复）。

## 相关

- Header / scope / sort / rename / delete 键位 → [`keybindings.md`](./keybindings.md)
- Spec 合约（斜杠与面板字段下限）→ `llmanspec` `app-tui-commands` atm10（本文件补布局细节，不另立平行斜杠语义）
