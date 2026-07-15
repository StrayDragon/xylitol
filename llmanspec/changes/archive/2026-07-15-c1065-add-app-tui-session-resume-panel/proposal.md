---
change_id: c1065-add-app-tui-session-resume-panel
title: "产品 /session-resume：对齐 pi Resume Session 面板（P0+P1+P2）"
status: full
priority: 1065
depends_on: ["c1020-add-app-tui-session-lifecycle-slash"]
author: agent
track: A
---

# c1065-add-app-tui-session-resume-panel

## Why

`/session-resume` 已能列表切换，但相对 pi `SessionSelectorComponent` 缺 header（scope/sort/name）、搜索语义、rename/delete/path、以及会话森林折叠。用户要求一次性对齐 P0+P1+P2（扫描中 live N/M 另议，见 future）。

## Purpose

将 `/session-resume` 从「扁 SelectList」升级为 **pi 对齐的 Resume 面板**（仍占 editor 槽，非 capturing overlay）：

### P0 — 面板壳与发现性

- Header：标题 + scope（Current Folder / All）+ Name filter + Sort。
- Tab：切换 scope（当前 cwd vs 全部会话目录树，语义对齐 pi；xylitol 会话布局以 Driver seam 暴露，禁止 TUI reach infra）。
- Ctrl+S（或键位表等价）：Sort 在 Threaded / Recent / Fuzzy(relevance) 间循环。
- Ctrl+N：Name 在 All / Named 间切换。
- 搜索框：支持 `re:<pattern>` 与 `"phrase"` 精确片段；普通 token fuzzy（对齐 pi `session-selector-search`）。
- 行：预览（name ?? first_message）+ message_count + 相对时间；Threaded 时带 parent 树前缀。

### P1 — 管理操作

- Ctrl+R：对选中项 rename（经 Driver `set_session_name` 或等价；可嵌 Input，Esc 取消）。
- Ctrl+D：删除确认（二次确认；**禁止**删当前活跃 session）；经 Driver seam 删除持久化会话。
- Ctrl+P：切换是否在行/描述中显示 path（或 cwd 短路径）。

### P2 — 森林折叠

- Threaded 模式下：对选中父节点折叠/展开子会话（键位对齐树侧 fold 习惯：ctrl/alt+left|right 或 design 钉死等价）；折叠后子行不可见但仍可 Enter 选中可见行切换。

### 不变式

- Enter 选定 → SwitchSession + 重建 transcript；Esc 关面板不切换。
- busy 拒绝打开；旧名 `/resume` 无效。
- TUI MUST NOT import `infra::session`。

## What Changes

1. Driver seam：列表字段补齐 cwd/path；`delete_session`；scope=all 时的列举（可跨 project dirs 若 xylitol 布局支持，否则 design 钉「单 sessions_dir + cwd 过滤」等价）。
2. 产品 `layout/session_resume`（或等价）面板组件替换纯 SelectList 挂载。
3. 键位：`app-tui-input` + design/keybindings。
4. harness 覆盖 P0/P1/P2 主路径；`PI_DELTAS` 记刻意差异（若有）。

## Capabilities

- `app-tui-commands`（修改 atm10）
- `app-tui-host`（列表/删除/scope seam）
- `app-tui-input`（面板键位）

## Out of scope / Future

- 扫描中 **live** `loaded/total` 逐文件刷新（用户确认可后做；见 `future.md`）。
- `/session-resume <id>` 直参；分享/远程同步删除。
- 解冻 Trust Choice stub。

## Ethics

- risk_level: medium
- prohibited_actions: TUI 直读/直删 sessions 目录；无确认删除；删除当前活跃 session
- required_evidence: harness + arch_guard + 键位表更新

## Depends

- c1020（已归档）：lifecycle + 列表预览/父指针基础
