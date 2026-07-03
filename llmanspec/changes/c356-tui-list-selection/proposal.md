---
change_id: c356-tui-list-selection
title: TUI 列表选择器（交互原子）+ 工具审批浮层
status: draft
priority: 356
depends_on: []
author: agent
---

# c356-tui-list-selection

## Why

当前 TUI 无法承载真实 agent 工作流——agent 工具调用需要审批时，TUI 没有 UI 接收用户决策（`XyEvent::ToolApprovalRequired` 到达后无交互入口）。`/model` 等命令也只能靠记忆输入，无可视选择。

三个对标项目都有「列表选择器」作为交互原子，picker/审批/设置类组件复用它：
- **codex**：`ScrollState`（`scroll_state.rs`）+ `selection_popup_common`（共享行渲染）+ `ListSelectionView`（`bottom_pane/list_selection_view.rs`）。审批浮层**组合** ListSelectionView（`approval_overlay.rs:607` 委托渲染）。
- **pi**：`SelectList`（`components/select-list.ts`，229 行）+ `SettingsList`（submenu/values 双模式）。
- **kimi-code**：`SearchableList<T>`（`utils/searchable-list.ts`）纯逻辑状态机（游标+分页+模糊），三个 picker（choice-picker/approval-panel/model-selector）复用。

本变更新增两类组件：`ListSelection`（交互原子，纯状态机 + 渲染）+ `ApprovalOverlay`（组合 ListSelection，接工具审批）。

## What Changes（草案）

1. 新增 `src/app/tui/components/list_selection.rs`：
   - 纯逻辑状态机 `ListSelection<T>`：`Vec<T> + selected_index + page_size`，方法 `move_up/down/confirm/cancel`，环形 wrap
   - 渲染 `render(area, buf, items, selected)` → 列表行 + `❯` 选中指针 + `(n/m)` 页码
   - 从 kimi-code 的 `SearchableList` 抽象起步（最薄），搜索能力留后续
2. 新增 `src/app/tui/components/approval_overlay.rs`：
   - 消费 UI-only 类型 `ApprovalRequest { call_id, tool_name, summary }`（由 `app.rs` 在 `XyEvent::ToolApprovalRequired` seam 翻译，组件不 match XyEvent，tui42）
   - 内部组合 `ListSelection`（选项：允许/拒绝/查看详情）
   - 选中后产出 `ApprovalDecision`，经 driver 回写 agent
3. Driver trait 补 `approve_tool(call_id, approved)` 方法（或经回调通道）——InProcessDriver 直接调 agent，RemoteDriver 发 REST

## Capabilities

- `app-tui`（修改）：新增交互组件约束（列表原子可复用、审批浮层组合列表、消费 UI-only 类型、TestBackend 可验证）

## Impact

- 新增 `src/app/tui/components/list_selection.rs` + `approval_overlay.rs`
- 可能改 `Driver` trait（加 approve_tool）、`app.rs`（XyEvent 翻译 seam）
- 风险：中。审批接线触及 agent 工具调用流程，需回归现有工具执行

## 调研证据（三家对比）

- **codex**：双 trait 分层（`Renderable` 纯绘制 + `BottomPaneView` 交互态机），活动视图栈 `Vec<Box<dyn BottomPaneView>>` 管理模态浮层。审批浮层不自己画列表，组合 ListSelectionView + `build_options` 翻译 ApprovalRequest→SelectionItem。键位经 `ListKeymap` 注入（不硬编码）。闭包 action 解耦（`SelectionAction = Box<dyn Fn(&AppEventSender)>`）。
- **pi**：`SelectList` 居中滚动窗口 + 双列布局（label/description）+ 环形 wrap + 确认走回调。`SettingsList` 的 submenu 模式（每项挂子组件）。
- **kimi-code**：`SearchableList<T>` 把游标+模糊+分页封成无渲染纯状态机，picker 只管「自有语义键」+ 委托「通用键」。审批面板的 `DisplayBlock` 判别联合（diff/shell/file_op/...）+ per-type 渲染。

**xylitol 取舍**：抄 kimi-code 的 `SearchableList` 纯状态机抽象（Rust trait 自然映射，最薄），抄 codex 的「审批浮层组合 ListSelection + build_options 翻译」，**不抄** codex 的双 trait + 视图栈（ratatui 用 Mode 枚举分发即可，不需 BottomPaneView 体系）。键位先硬编码（kimi-code 的 keymap 注入留后续）。

## 不在本变更范围

- 命令面板（`/` 触发的补全）——c357
- 模型选择器——可复用 ListSelection，但单独变更
- markdown 渲染（审批「查看详情」可能需要）——c355 先行，本变更依赖其 finalize 后渲染
- 键位自定义/keymap 注入——后续
- 搜索/模糊（SearchableList 的 searchable）——后续

## 约束草案（full 化时落 spec.toon）

- ListSelection MUST 是纯逻辑状态机（无渲染依赖），渲染单独函数，两者皆可独立测试
- ApprovalOverlay MUST 组合 ListSelection（不重写列表渲染），消费 UI-only `ApprovalRequest`
- 审批决策回写 MUST 经 Driver trait（不直接调 agent），保持 tui4 复用契约
- 所有交互组件 MUST 可经 TestBackend 验证状态转换 + 渲染输出（tui41）
- 交互组件 MUST 消费 UI-only 数据类型，不 match XyEvent（tui42）
