---
depends_on: []
title: Anthropic SSE 坏帧连续超限终断
branch: sdd/c3339-fix-anthropic-sse-corrupt-guard
base_branch: main
base_sha: b886049fb82f902965fb08df5d4d4cbbbfeec11b
---

## Why

anthropic_stream 对 SSE 帧解析失败与 data JSON 解析失败一律 continue（静默丢帧）。损坏的流可能持续吞帧直到 SSE_IDLE 超时才暴露，期间 delta 无声丢失，排障无计数可查。审计定级中风险 M6，用户决策走 propose 一步到位。

## What Changes

- anthropic_stream 引入连续坏帧计数：每次解析失败 warn 告警；连续达 3 帧以 XyError::Provider 终断流；成功解析归零（散发坏帧仍跳过续流）
- infra-provider spec 新增规则 anthropic-sse-corrupt-frame-guard（r3309）
- 插件单测：连续坏帧终断、散发坏帧可恢复两例（wiremock 回放）

## Impact

- 行为变化仅限「连续 3 帧坏帧」的流：原先静默吞到 idle 超时，现在快速终断并带错误语义；终态约束由 chunk_stream_check 既有矩阵覆盖
