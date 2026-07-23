---
version: "alpha"
name: "session-resume"
description: "/session-resume picker — proportional preview; Ctrl+U toggles full session id."
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

### 默认（id 隐藏）

| 列 | 内容 | 宽度规则 |
|---|---|---|
| **预览** | `name`，否则 `first_message`；皆空则 `—`（**不**用 id 顶替预览） | 扣除 meta 后剩余宽度，**软顶约终端宽 60%**；超长 `…` |
| **元数据** | `message_count` + 相对时间（age） | 右端块 |

### id 显示（`Ctrl+U` / `app.session.toggleId` 打开后）

在预览与 meta 之间插入 **完整 session id** 列：

| 规则 | |
|---|---|
| 完整显示 | **MUST NOT** 截断 UUID/id；视口内按最长 id 对齐 |
| 窄终端 | 优先压缩预览；宁可整行略超宽，也不得吃掉 id |
| 再按 `Ctrl+U` | 隐藏 id 列，预览重新吃回空间 |

选中行：`›` 前缀 + `{colors.user-message-bg}` 整行 wash（**MUST NOT** reverse-video）。

视口：最多约 10 行可见 + `(i/n)` 滚动提示（与 pi SessionList 对齐）。

Header 提示 MUST 含 id 开关和弦（如 `(ctrl+u)`），与 path `(ctrl+p)` 同类。

## MUST NOT

- 默认常显 id 挤占预览（排障用 toggle，不默认占位）。
- 显示态为塞满预览而截断 id。
- 在无 name/`first_message` 时把 id 同时画进预览列与 id 列（重复）。

## 相关

- Header / scope / sort / rename / delete / id toggle 键位 → [`keybindings.md`](./keybindings.md)
- Spec → `app-tui-commands` atm10 · `app-tui-input` ati29
- 静图 → [`playground/`](./playground/) Resume 槽（默认 / id-on）
