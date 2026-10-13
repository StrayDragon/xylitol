---
depends_on: []
title: 固定区组件命名统一：覆盖弹出框 / 侧栏（代码与文档同源）
needs_specs_change: true
branch: sdd/c2913-refactor-popup-sidebar-naming
base_branch: main
base_sha: 835f95b89fd1dd2ae8a5d3a921c933e03b1cc274
---

# 固定区组件命名统一：覆盖弹出框 / 侧栏（代码与文档同源）

## Why

c2875（瞬态会话详情浮层）与 c2905（常驻右列）交付时的组件名「抽屉 / 驻列」未经固定区词汇表注册，用词与形态不匹配（浮层只覆盖对话条目区域，并非抽屉；右列即通用 panel 形态）。2026-10-13 词汇定案：**覆盖弹出框**（overlay popup）/ **侧栏**（sidebar · panel），已录入 [TUI信息呈现与固定区词汇.md](../../docs/architecture/TUI信息呈现与固定区词汇.md) 与 [术语表.md](../../docs/architecture/术语表.md)（弃用对照已建）。本 change 把代码标识符与活 spec 措辞统一到该真值（需求语义零变化）。

## What Changes

- 代码标识符：`session_drawer*` → `session_popup*`（含键位动作 `app.session.drawer` → `app.session.popup` 与英文描述）；`sidebar` 标识符已符合，仅清理注释旧词「驻列」。
- 活 spec：`app-tui-fixed-zone` r2903「会话详情抽屉」→「会话详情覆盖弹出框」、r2904「会话侧栏驻列」→「会话侧栏」，规则文本与场景名同步（`session-drawer-*` → `session-popup-*` 等），BDD 绑定同步。
- 文档：`src/app/tui/DESIGN.md` 硬规则 2 例外注记改新词并修正陈旧键位（alt+d → alt+i，实际键位以 keybindings 为准）；c2911 草案改新词；术语表标识符行刷新。
- designing 补两个已交付组件的模块稿（`session-popup` / `session-sidebar`，含 states），补齐 c2875 / c2905 / c2912 漏登记。

## 非目标

- 不改任何行为（渲染、键位绑定、几何、场景断言内容均不变；仅名称与注释）。
- 不动归档 change 史实文（旧词按词汇表维护规则保留）。
- 不做侧栏业务字段回归（c2911 草案候选，另行启动）。
