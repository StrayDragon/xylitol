---
depends_on:
  - c3006-add-minimal-eval-harness
title: "Add eval fast regression (golden + replay + snapshot)"
needs_specs_change: false
---

# Add eval fast regression（golden + replay + snapshot）

## Why

c3006 已交付 5-task DeepSWE typical set，但每次回归都要调用真实模型，成本高、受 quota 与网络影响。
改系统提示、工具策略或 session 序列化后，缺少一个不烧 token 的可重复检查；问题往往到下一次
全量 eval 才暴露。

## What Changes

- 新增 3–5 个 golden fixture：小仓库 + 明确 bug + 测试 + expected patch/test。
- 新增 mock/replay OpenAI-compatible server：回放录制 session JSONL，工具真实执行。
- 新增 session snapshot：归一化 timestamp / session id 后比较关键事件与最终 patch。
- 新增本地 `just eval-regression`：无模型调用、无外网；失败打印差异。
- 保持与 c3006 同一 binary / config / print 路径；不引入第二套 agent 驱动。

## Scope

- golden fixture 覆盖：工具调用、文件编辑、测试失败重试、session 恢复。
- replay server 只服务本地 fixture；不接真实模型。
- snapshot 只比较稳定字段；不稳定字段列在 normalization 清单。
- 失败实例可以提升为 golden；提升规则写进 `tools/eval/README.md`。

## Non-goals

- 不做 CI / PR 门禁；本 change 只提供本地可跑入口。
- 不做 Langfuse、云并行、全量 eval。
- 不替换 Harbor verifier；golden 是快速回归，不是分数源。
- 不引入新的产品行为或 live spec。

## Acceptance

- `just eval-regression` 在无模型、无网络环境下通过。
- 同一 fixture 连续跑两次，snapshot 稳定。
- 故意改坏一个工具调用或 patch，回归能报出具体差异。
- 不改变 c3006 的 5-task typical set 与报告口径。

## Risks

- replay 录制会随 prompt 变化漂移；需要固定 fixture 版本与 normalization 清单。
- snapshot 容易脆；只比较稳定字段，避免 timestamp / 随机 id。
- golden fixture 维护成本；控制在 3–5 个，不堆场景。

## Further Notes

- 依赖 c3006 的 `tools/eval/`、`xylitol print`、binary 与 5-task baseline。
- 后续如果要做 CI 门禁，另开 change；本 change 不碰 CI。
