---
change_id: c545-update-package-tui-editor-sources
title: "package-tui-editor-autocomplete：补全源扩展钩子与窄宽 popup"
status: full
priority: 545
depends_on: []
author: agent
track: P
---

# c545-update-package-tui-editor-sources

## Why

CompletionSource 注册表已支持 `/` 与 `@`；轨 P 需要把「未来 `$`/`^`」落成**可注册、默认可空**的扩展点，并保证窄终端下补全 popup **不撑破宽度预算**，避免 demo/产品开闸后再打补丁。

## Purpose

增补 `package-tui-editor-autocomplete`：扩展源契约 + 窄宽 popup 宽度不变量。

## What Changes

1. 文档化/测试：额外 CompletionSource 可注册且默认可不装（开闭）。
2. 窄宽下 SelectList popup MUST 遵守 Editor 内容宽（截断/夹紧，不溢出）。
3. 扩既有 completion / editor harness 用例。

## Capabilities

- `package-tui-editor-autocomplete`（修改）

## Impact

- `packages/xylitol-tui/src/completion.rs`、`components/editor.rs`、相关测试
- 不实现产品真 `$`/`^` 业务语义（仅扩展点）

## Out of scope

- 产品 slash 真命令 / Driver 接线（轨 B）
- 重写 CombinedAutocompleteProvider 以外的历史 API（可保留 shim）

## Ethics

- risk_level: low
- prohibited_actions: 不在包内硬编码产品命令表
- required_evidence: completion/editor 相关测试绿；validate 通过
