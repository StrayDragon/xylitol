---
depends_on:
  - c3110-add-session-stage-name
title: TUI 阶段名展示与首轮触发
needs_specs_change: true
---

# TUI 阶段名展示与首轮触发

## Why

c3110 提供阶段名生成与持久化，但产品 TUI 还没有展示位、首轮触发与手动改名/刷新入口。用户要求阶段名显示在编辑区上边框，替代已下线的「详情」overlay 所承担的会话身份可见性；默认只在首个会话首轮自动生成一次，改名或需要时再触发。

## 术语

- **阶段名**：当前会话的短显示名（session display name 在 TUI 的称谓）；显示在编辑区上边框。
- **首轮触发**：fresh 会话首个 `AgentEnd` 后，若本地无阶段名，调用一次 `generate_session_name(force=false)`。

## What Changes

- `UiModel` 增加 `stage_name: Option<String>`；`UiRoot` 同步；编辑区上边框**右端**绘制阶段名（右对齐，不居中；muted，超宽 `…` 截断，不换行）。
- attach / resume 后经既有 `get_session_name` 读取；若已加载 entries，可从 `SessionInfoEntry.name` 直接取；不触发自动生成。
- fresh 会话首个 `AgentEnd` 后，若阶段名为空，异步调用 `generate_session_name(force=false)`；每个会话最多一次；不阻塞输入/主回复。
- 新增 slash `/stage-name`：
  - `/stage-name`：显示当前阶段名（或提示未生成）。
  - `/stage-name <text>`：显式改名（走既有 `set_session_name`，钉住）。
  - `/stage-name refresh`：显式刷新（`generate_session_name(force=true)`）。
- remote driver 增加 `exec_generate_session_name`；in-process 直接调 `XyDriver`；协议 `Command::GenerateSessionName { force }` 由 c3110 定义。
- fork 后读取父名并显示；不自动重命名。subagent 不显示/不生成。
- 失败：host 返回回退名；TUI 只显示返回名，不向用户抛错；生成中不显示假 Busy。

## Capabilities（拟）

- `app-tui-fixed-zone`：编辑区上边框阶段名展示、截断、无阶段名时不占位；与 chip 同高不增高。
- `app-tui-commands`：`/stage-name` 查看 / 改名 / 刷新；与既有 session resume rename 语义一致。
- `app-tui-input`：slash 解析与 pending 形状（若命令表需要）。
- 复用 `agent-session` 的阶段名语义；不在本 change 重复定义生成策略。

## Impact

`crates/xylitol-app-tui` 的 bridge / layout / host / commands / driver remote / tests；不改 runtime / protocol 生成实现（c3110）。视觉上编辑区上边框新增一段 muted 文本；c3130 会继续调整 chip / 字形 / 圆角，本 change 只保证阶段名有稳定落位。

## 非目标

- 不做阶段名生成策略 / 模型调用 / 持久化（c3110）。
- 不做「详情」overlay 下线（c3130）。
- 不做折叠等级 chip / `/fold`（c3120）。
- 不做 resume 列表批量重命名；只做当前会话。
- 不做多客户端实时同步；以 unary 返回为准。

## 状态注记

> **designed**：proposal + design 已完成；未写代码、未绑分支、未改 live specs。apply 时按 design 的「实施任务」生成 tasks.md。
