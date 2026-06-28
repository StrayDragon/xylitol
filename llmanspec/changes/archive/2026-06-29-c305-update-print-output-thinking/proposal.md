---
change_id: c305-update-print-output-thinking
title: print 模式 thinking 输出与 MessageUpdate 去重写入 spec
status: proposed
priority: 305
depends_on:
  - c300-add-provider-adapter-layer
author: agent
created: 2026-06-29
---

# print 模式 thinking 输出与 MessageUpdate 去重写入 spec

## 背景与动机

在修复 thinking/reasoning 展示问题的过程中，commit `03f8866` (`fix(print): handle ThinkingDelta and avoid double MessageUpdate output`) 直接给 `src/app/print.rs` 引入了两项行为变更，但**没有走 SDD 流程、也没有更新 `print-output` spec**：

1. `ThinkingDelta` 现在会流式输出到 **stderr**，并用 `<think>...</think>` 包裹；当模型自身已带 `<think>` 标签时不再重复包裹。
2. `MessageUpdate`（累计完整消息状态）不再写 stdout，避免 `HelloHello!Hello! How...` 这种前缀重复输出。

`print-output` spec 当前只记录了"streaming text to stdout"和"tool display"，对 thinking 输出只字未提，spec 与实现已经脱节。本变更把这两项行为补回 spec，使文档与代码一致。

注：这是对既有 commit 的 spec 补登记，不含代码改动。

## 变更内容

更新 `print-output` spec：

- **新增 requirement**：print 模式必须把 reasoning/thinking 内容流式输出到 stderr（不污染 stdout），并用 `<think>...</think>` 标记；需兼容模型自带 think 标签的情况。
- **新增 requirement**：print 模式只能把增量 `TextDelta` 写到 stdout，不得把累计型 `MessageUpdate` 当作增量重复写入。

## 受影响能力

- `print-output`（更新）：补充 thinking 与 message-dedup 行为。

## 影响面

- **代码**：无改动（行为已在 `03f8866` 实现）。
- **测试**：`src/app/print.rs` 已有 `thinking_delta_does_not_pollute_stdout`、`thinking_delta_with_embedded_tags_is_not_double_wrapped`、`text_delta_only_is_written_once` 三个测试覆盖。

## 验收标准

- `llman sdd validate c305-update-print-output-thinking --strict --no-interactive` 通过。
- `print-output` spec 的 requirements 与 `03f8866` 的实际行为一致。
