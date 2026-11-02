---
depends_on: []
title: 流式思考块可折叠且增量不丢失
branch: sdd/c3031-update-streaming-thinking-fold
base_branch: main
base_sha: df562ba5888bd110063d2480c6f94fc43efdc15d
---

## Why

流式 thinking 正文被合约强制保持展开：点折叠三角只翻标记，正文仍在涨。这是呈现策略，不是数据流约束。用户习惯是「正在输出的块也能折」，折起来后新增量继续进缓冲、展开后仍在。

## What Changes

- 流式 thinking 默认展开；点该块头写入 per-id 覆盖，正文按有效态显示或收起。
- 折叠只改端侧呈现：`streaming_thinking` 缓冲与 Host 事件流不因折叠丢弃、截断或回放。
- 新 delta 不得把用户收起的块重新展开。
- 流式 id `live:streaming` 的覆盖在通道结束后迁到落地 Thinking id；仅 thinking 的簇在用户明确展开时打开簇，未触碰则仍默认收成 Thought 头。
- 与工具同簇且簇折叠时，仍以独立 Thinking 行呈现当前输出；该行自身可折，簇折叠不得强制展开或强制收起该行正文。
- 不改 Host / 线协议 / 会话落盘。

## Capabilities

- `app-tui-transcript`

## Impact

- 产品 TUI 流式 Thinking 头可点收起；结束通道后折叠有效态延续。
- 既有「流式期间必须展开」条款改为默认展开、用户可折。
