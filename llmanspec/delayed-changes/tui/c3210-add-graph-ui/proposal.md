---
depends_on:
  - c3206-refactor-session-graph
title: "Add session graph UI"
---

## Why

c3206 提供图数据（nodes / edges / branches / heads），但 TUI 仍是树视图，nsh 原生端没有图可视化。人主导的分叉、连线即上下文、禁用污染子图，需要图界面。

## What Changes

- session_graph 协议数据：nodes、edges、branches、epoch。
- TUI：树视图改为图数据渲染（MVP 仍树形布局）；fork / travel 走图边。
- nsh 原生端：画布节点 + 连线；新建 context 边、禁用边、fork；上下文预览。
- 不复制 ThoughtDAG UI；复用我们的图模型与协议。

## Scope

- 不在 c3206 内实现；c3206 只提供图数据与协议字段。
- 先 TUI，后 nsh；nsh 先 spike。

## Related

- #74（nsh 原生端，M1 spike 先做）
- #73（TUI 使用痛点）
- #12 / #11（gpui 方向，已关闭；nsh 合并历史）

## Open Questions

- 图的布局与规模上限。
- 连线编辑权限：用户可编辑，agent 是否可写。
- 与 fork / travel 的交互一致性。
