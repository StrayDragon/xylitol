---
change_id: c1605-add-runtime-prompt-fragments
title: 运行时能力 ↔ 系统提示片段自动注入（少配置面）
status: in-progress
priority: 1605
depends_on: []
author: agent
branch: feat/c1605-c1610-oob-prompt-batch
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: false
---

# c1605-add-runtime-prompt-fragments

## Discussion context（2026-07-24）

见 `c1610` 实验 3：固定多-tool 提示显著提高同消息多 tool 命中率。本 change 提供机制；默认内容与默认 `barrier_parallel` 由 `c1610` 打开。

## Why

能力/策略变更应自动带上匹配提示，避免用户手贴 APPEND。

## Decisions

1. 内置 fragment 表（id + body）；`fragments_for_batch_mode(mode)` 解析。
2. `SystemPromptOpts.runtime_policy_fragments: Vec<String>`；`build_system_prompt` 注入 `<runtime_policy>`（在 guidelines 前、append_system 后或 guidelines 后——定：在 Guidelines 段之后、date/CWD 之前）。
3. Session：`batch_mode` 变更与构造时同步 fragment 列表并 `rebuild_system_prompt`。
4. 无用户 YAML `prompt_fragments` 配置节。

## Status

**promoting / applying** with `c1610` on same feature branch.
