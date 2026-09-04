---
depends_on: []
---

# 延后：跨面公共动作 id 同源（client 登记表，不进 Host）

> **⚠️ deferred（2026-08-21）**：移入 `llmanspec/delayed-changes/tui/`，避免活跃 graph 里像还要落地。TUI 折叠动作已在面本地交付；Web 未开闸。**禁止**做成 Host unary / 服务端执行器。文档只写现状，不为未开闸面预埋第二套词表。

约束板：`docs/roadmaps/Web与TUI同源.md`。TUI 已有 `activity.expandNearest` / `activity.collapseNearest`（归档 c1760）。Web 未兑现。本票记住：凡两面都会有的能力，动作 id 与学习成本 MUST 一套；键位 SHOULD 同构。面本地（TTY 选区 / DOM 点击）可分叉。实现只在 TUI / 未来 Web 这类 client；Host 只给会话事实与事件。

## Why

Host 信封（c2290）管会话/工具，不管框架动作。若不单独记账，Web 开闸时容易另起一套「展开/折叠」故事，违反跨面同源。

## What Changes

- 公共动作登记表（id 稳定、语义中文、哪面已兑现）。起步至少：折叠栈 expand/collapseNearest；后续即时设置覆盖、改道等按 roadmap M1/M1b 加行。
- Web UI 未开闸前可以只交清单 + spec 意图，不画页面。
- 禁止把 TUI 专属键帽写成 Web MUST；禁止为 Web 发明第二套公共 id。

## Capabilities

- 跨面同源词汇（落地时再钉 capability；现约束在 roadmap）

## Impact

不改 Host 方法表。TUI 已交付的动作 id 保持；Web 日后必须复用。

## 非目标

实现 Web UI、改 Host RPC、像素同貌、方案 A。
