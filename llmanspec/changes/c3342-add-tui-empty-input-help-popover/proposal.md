---
depends_on: []
title: 空输入提示行与快捷键面板
---

# 空输入提示行与快捷键面板

## Why

- 产品 TUI 输入框为空时没有可发现入口；键位与 20+ slash 命令靠记忆。
- 用户要求模仿 Codex：footer 路径行下方加一行提示，空输入显示，有内容隐藏。
- 按 `?` 打开快捷键面板；面板列键位与功能说明。
- 与 #73「TUI `/help` 入口与键位发现」条目同源。

## What Changes

- footer 下方新增第二行 hint：空输入且面板关闭时显示 `? shortcuts`；有内容时隐藏。
- 面板打开时 hint 行显示 `? / Esc close`。
- 空输入 + `?` 键 = 打开快捷键面板；`?` 不得写入输入。
- 输入非空 + `?` 键 = 输入字面 `?`。
- 面板打开时 `?` 或 Esc 关闭；面板占据 transcript 区，输入框仍在底部。
- 面板内容由 keybindings 注册表动态生成；按宽度分 3/2/1 列；内容过高时可滚动。
- MVP 面板只读；点击行触发功能不在本期。
- `/help` slash 命令不在本期。
- 不新增第二套动作词表；键位与描述来自 keybindings 注册表。

## 已钉决策

- hint 文案只写 `? shortcuts`。
- 面板内容动态 keybindings。
- 面板只读；点击行留到后续。
- busy 时可打开面板；Esc 先关面板，不 abort。
- 窄终端 3/2/1 列。

## 现状与约束

- footer 是最后一行 dim 文本（`crates/xylitol-app-tui/src/widgets/mod.rs:90`）；dock 组合在 `crates/xylitol-app-tui/src/layout/root/render.rs:495`。
- `last_dock_rows` 与 `editor_top_row` 必须把 hint 行计入（`render.rs:123`、`:520`）。
- `Editor` 空文本无占位符 API（`crates/xylitol-tui/src/components/editor/layout.rs:315`）；本方向不改 Editor。
- 键位表带描述（`crates/xylitol-app-tui/src/keybindings.rs:10`）；`KeybindingsManager::get_keys` 可取当前键位（`crates/xylitol-tui/src/keybindings.rs:210`）。
- 树槽已有动态 help 行（`crates/xylitol-app-tui/src/layout/session_tree.rs:92`）；可复用键位格式化。
- `app-tui-fixed-zone` 的 `footer-minimal` 规定 footer 1 行；hint 行是独立第二行，必须同步 spec。
- `app-tui-fixed-zone` r1228 的 Fixed-Zone Footprint 必须加入 hint 0/1 行。
- 空输入按 `?` 会与「输入字面 ?」冲突；`?` 仅在空输入时被拦截。

## Out of scope

- 点击 hint 或面板行触发动作。
- `/help` slash 命令。
- 鼠标点击 hint。

## Related

- #73 TUI/UX 痛点
- `app-tui-input`、`app-tui-fixed-zone`、`app-tui-commands`
