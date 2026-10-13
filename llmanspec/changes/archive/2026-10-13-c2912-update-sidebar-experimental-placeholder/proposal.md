---
depends_on:
  - c2905-add-tui-session-sidebar
  - c2910-fix-tui-sidebar-scroll-region
title: 驻列改实验占位（施工中），会话详情字段延后
branch: sdd/c2912-update-tui-sidebar-placeholder
base_branch: main
base_sha: f810e673a2fbc0809ad2435ad13a5a7b642fa1b9
---

# 驻列改实验占位（施工中），会话详情字段延后

## Why

实机结论：现驻列内容（会话身份/模型/thinking/目录）并非目标用途——正式业务字段未定。驻列机制本身（让宽/分区/贴靠）已按真实业务试错收敛，保留为实验特性；内容降级为施工中占位，字段待正式业务代码定义后回归（候选入 c2911 草案）。

## What Changes

- `session_sidebar_cell` 字段区（会话/模型/思考/目录 4 行）→ 实验占位（「实验特性 · 施工中」「正式业务后启用」）；标题「会话侧栏」、键位、24 列几何、c2910 边界语义全部不动。
- r2904 规则：「关注字段（…4 项）」→「实验占位内容（施工中；业务字段待正式业务代码定义）」。
- 字段底层数据（`session_drawer_id` 等）为抽屉共用，保留不动。

## Capabilities

- `app-tui-fixed-zone`（r2904 文案收窄；场景不变仍绿）

## Impact

仅 `render.rs` 单函数与规格文案；抽屉 c2875 不受影响。
