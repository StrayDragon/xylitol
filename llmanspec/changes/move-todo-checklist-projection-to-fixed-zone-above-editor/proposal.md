---
depends_on: []
---

## Why

Todo checklist 投影行（`Todo · N/M` 折叠行）目前是**对话区内**的 latest-wins 行：
每次 `TodoUpdated` 事件把旧行删除、在 transcript 尾部重新插入。位置因此随对话
推进漂移——插入后若助手继续输出正文/工具，清单行会被推离视口焦点；它表达的是
「当前任务状态」，本质是**常驻状态**而非对话历史，放对话区内语义与可用性都不佳。

用户实测反馈（2026-09-13，refine-todo-display-planes 归档后目检）：清单行更适合
放**输入框上方固定区**——像队列条/状态条一样常驻、稳定、不必在对话流里找。

## What Changes

- Todo checklist 从对话区 UiEntry::Todo 行迁移为**输入框上方 fixed zone 常驻
  卡片**：默认一行摘要（`Todo · N/M`），可展开完整勾选清单；有清单时驻留、空表
  消失。
- 对话区 `todo_*` 工具块（时间线痕迹）**保留不动**；`UiEntry::Todo` 行取消。
- 数据源不变：仍从 `TodoUpdated` 类型化事件（live）与 `agent_todo` SSOT 快照
  （resume）取数——只动呈现位置，不动信息流。
- 折叠/展开交互、与既有固定区（队列条、toast、状态条）的堆叠次序与词汇遵循
  `app-tui-fixed-zone` 与 TUI 信息呈现固定词。

## Capabilities（预估，正式化时核对）

- `agent-todo`：atd8（对话区可折叠 checklist）需修订为固定区形态；atd9 语义
  基本保留（resume 重建数据源不变）。
- `app-tui-fixed-zone`：新增固定区成员（todo 卡片）的布局/折叠/热键条款。
- `跨端同源` 约束板：gpui 桌面端的对应固定区语义需同款。

## Impact

- 行为合约变更（atd8 MUST 条款改写）→ 走完整 SDD pipeline（propose 起）。
- 涉及 TUI 布局固定区、折叠命中、differential render；需 `designing/` 模块
  （activity-fold 的 todo-bar 描述与 tool 模块指针同步更新）+ `just
  export-design-frame`。
