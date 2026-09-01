---
depends_on: []
skip_specs_landing: true
branch: sdd/c2500-unify-select-list-protocol
base_sha: 50095ac9a0b48d08e133dc07fd0805362ad18e7c
checkpointed: false
---

# SelectList 结构下沉：五槽去重与过滤纯函数化（行为零变化）

## Why

包层已有 `SelectList/SelectItem` 组件且被五个槽使用（import / models /
themes / mcp / resume），但各槽**各自组装**：构造样板、布局/截断上下文、
过滤逻辑存在重复；过滤语义已有分叉（models 走包层 `fuzzy_filter`，
`SelectList::set_filter` 是前缀匹配）。

**2026-09-01 拍板（深挖后降级）**：现有行为一致且好用，UI 语义变更全部出局——
原提案的分组头 / details 多行 / action 条 / Tab 焦点环 / opencode 式上下文命令 /
resume 迁移**均不做**。本票降级为**纯代码结构下沉重构：行为零变化**。
（交互机制调研沉淀见 `research/pi-opencode-interaction-notes.md`，供 c2505 与未来票参考。）

## What Changes

- 下沉五槽重复的 `SelectList` 组装样板（构造参数、主题、布局/截断上下文构造）到共享路径；
- 过滤逻辑抽纯函数进包测试层；**各槽既有过滤语义保留**（fuzzy 与前缀匹配的差异参数化，
  不统一、不改行为）；
- 布局/截断辅助去重（`SelectListLayoutOptions` / `TruncatePrimaryContext` 复用路径收敛）；
- 包层公开 API **不加任何新字段**（group / details / action 一律不加）。

## 非目标

- **resume 自绘 panel（812 行）完全不动**（含 search / 分批 / rename / delete）；
- 不改任何键位、渲染、过滤行为——所有槽的既有交互决议保持（PI 对齐项不变）；
- 不做分组头、details、action 条、命令化；c2505 命令面板仍直接复用包层
  `SelectList` + `fuzzy_filter`（已存在，无需本票新增能力）。

## Impact

- `packages/xylitol-tui/src/components/select_list.rs` 与五槽接线点的等价重构；
- **无 specs 合约变更 → `skip_specs_landing: true`**（行为不变）；
- 护栏：五槽 harness 快照不变（重构前后快照必须零 diff）+ 包层单测；
- designing states 无需更新（无 UI 变化）。

## 决策记录

- 2026-09-01（深挖拍板）：行为保留、UI 变更出局、resume 不动；`../pi` 与 `../opencode`
  交互机制调研归档至 `research/pi-opencode-interaction-notes.md`；
- 2026-09-01：过滤语义差异（fuzzy vs 前缀）**保留并参数化**，不作为统一对象。

## Further Notes

- 现状核对（含 API 清单与使用分布表）：[research/select-list-protocol-notes.md](./research/select-list-protocol-notes.md)
