---
change_id: c490-add-app-tui-trust-prompt
title: "app-tui 首次目录 trust 选择器（对齐 pi）"
status: purpose-draft
priority: 490
depends_on: ["c460-add-app-tui-host"]
author: agent
track: B
---

# c490-add-app-tui-trust-prompt

> **status: purpose-draft**（可与垂直切片并行；建议 c485 前或紧后）

## Purpose

未信任且存在 trust inputs 时，TUI 弹出选择器（替换 editor 槽或 overlay），写入 trust store；信任后 yolo 执行工具（保留 hook 扩展点，无逐工具审批）。

## Notes

- `infra/trust` 已有 Ask + `on_prompt` 回调；bootstrap `interactive: true` 接线。
