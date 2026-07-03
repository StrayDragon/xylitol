---
change_id: c357-tui-command-popup
title: TUI 命令面板（/ 触发的 slash 命令补全）
status: draft
priority: 357
depends_on: []
author: agent
---

# c357-tui-command-popup

## Why

当前 TUI 的 slash 命令（`commands.rs`）只能靠用户记忆完整输入（`/exit` `/model`），无可视补全。`c336` 已让 dispatch 就绪（compact/export/session 等命令经共享 dispatch 可执行），但用户不知道有哪些命令、参数怎么填。

三个对标项目都有「命令面板」——用户输入 `/` 触发下拉，列出可用命令 + 描述，选中后执行或补全参数：
- **codex**：`CommandPopup`（`bottom_pane/command_popup.rs`），内嵌于 chat composer，按 `/` 后首 token 过滤，exact/prefix 匹配 + 高亮 match_indices。不独立成浮层 view，是 composer 的一部分。
- **pi**：无独立命令面板，靠 `SettingsList`（submenu 模式）承载设置类命令。
- **kimi-code**：复用 `SearchableList` + 命令列表，数字键直选。

本变更新增 `CommandPopup` 组件，输入 `/` 触发，列出 `Driver::get_commands()` 返回的命令（c336 已在 dispatch 就绪），选中后补全到输入框或直接执行。

## What Changes（草案）

1. 新增 `src/app/tui/components/command_popup.rs`：
   - 状态：`filter: String`（`/` 后的文本）+ `commands: Vec<CommandInfo>`（来自 `Driver::get_commands()`）+ `selected_index`
   - 渲染：下拉浮层，列出 filtered 命令（name + description），选中项高亮
   - 交互：`/` 触发显示，输入字符更新 filter，↑↓ 移动，Tab/Enter 补全到输入框，Esc 取消
2. 接入 `input.rs`：检测 `/` 开头时唤起 CommandPopup，键位路由到 popup（与正常输入互斥）
3. 命令执行：补全后用户仍需 Enter 提交（走现有 `commands.rs::dispatch`），或 Enter 直接执行（看 full 化时决策）

## Capabilities

- `app-tui`（修改）：新增命令面板约束（/ 触发、消费 CommandInfo、组合 ListSelection、TestBackend 可验证）

## Impact

- 新增 `src/app/tui/components/command_popup.rs`
- 改 `input.rs`（`/` 检测 + 键位路由）、可能改 `mod.rs`（事件循环加 popup 状态）
- 风险：低。纯 UI 增强，不动 agent/dispatch

## 调研证据（三家对比）

- **codex**：CommandPopup 是 chat composer 内嵌下拉（非独立浮层），`on_composer_text_change` 解析 `/` 后首 token 设 filter，`filtered` 分 exact/prefix 匹配记录 `match_indices`。复用 `selection_popup_common` 行渲染。选择结果回传 composer 处理。
- **pi**：无独立命令面板，靠 SettingsList 的 submenu 模式（每项挂子组件）承载设置。
- **kimi-code**：choice-picker 模式，数字键直选。

**xylitol 取舍**：抄 codex 的「composer 内嵌下拉 + filter 匹配」，组合 c356 的 ListSelection（不重写行渲染）。xylitol 的命令集来自 `Driver::get_commands()`（c336 已暴露），无需另建命令注册表。

## 不在本变更范围

- 命令参数补全（`/model <id>` 的 `<id>` 补全）——后续，依赖模型选择器
- 命令历史/快捷键绑定——后续
- fuzzy 搜索——后续

## 约束草案（full 化时落 spec.toon）

- CommandPopup MUST 在 `/` 开头时触发，消费 `Driver::get_commands()` 返回的 CommandInfo
- MUST 组合 ListSelection（不重写列表渲染，复用 c356）
- filter 匹配 MUST 支持 prefix（exact 可选）
- 键位路由 MUST 与正常文本输入互斥（popup 激活时吞键）
- MUST 可经 TestBackend 验证（filter/选中/补全，tui41）
- MUST 消费 UI-only 数据（CommandInfo），不 match XyEvent（tui42）
