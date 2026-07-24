---
change_id: c1540-update-tool-timeout-unlimited
title: 工具与 Hook 超时默认无限；用有语义类型表达限时
status: proposed
priority: 1540
depends_on: []
author: agent
branch: feat/c1540-tool-timeout-c1545-batch-mode-draft
base_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
checkpointed: true
checkpoint_sha: 7c5075601088d1bfe49a865c31b80aa7bfaa24bf
---

# c1540-update-tool-timeout-unlimited

## Why

对齐 `../pi` 的工具超时心智：**默认不限时**，仅在显式传入正数秒时才限时。xylitol 现状不一致且双路径分裂：

- bash schema 默认 30s、max 120s；LLM 流式路径 `execute_streaming` **丢弃** timeout，改走 `InfraBashExecutor` 硬编码 30s
- grep / find 硬编码 30s，不可配置
- Hook 脚本 `timeout_secs` 默认 5s，且 dispatcher `max(1)` 强制至少 1s

用 `0` 表示「无限」缺乏 Rust 语义；应用 `Option` / 专用枚举表达「未设置 = 无限」。

## What Changes

- **bash**（含流式与非流式 / bang `InfraBashExecutor`）：默认无限；可选正整数秒；流式 MUST 尊重同一套解析
- **grep / find**：schema 增加可选 `timeout`；默认无限
- **Hook**：`timeout_secs` 改为可选；缺省 = 无限；去掉「至少 1s」钳制（有值须为正）
- **类型**：内部 `Option<Duration>` 或 `ToolTimeout { Unlimited | After(Duration) }`；省略字段 = 无限；**禁止**把 `0` 当无限（非法 → 参数错误）
- 保留合理 **max 上界**（防溢出，与默认无限正交）；建议对齐现有 bash max 120s 量级或 pi `MAX_TIMEOUT_SECONDS`，在 design 钉死
- Provider HTTP / OTEL 等非工具执行超时 **不在范围**

## Capabilities

- `agent-tools`
- `agent-hooks`
- `infra-bash`

## Impact

- **Breaking（行为）**：长跑命令/hook 不再被默认 30s/5s 默杀——需 abort 或显式 timeout
- 更新依赖默认超时的单测；BDD 保留显式 timeout 场景，并覆盖非法 timeout / 可选参数

## Out of Scope

- 工具批次并行 → `c1545`
- 内存 / CPU 配额

## Open Questions（propose 已收敛）

- bang / `InfraBashExecutor` 与 LLM bash **共用**默认无限语义：是
- max 上界：保留（具体秒数见 `design.md`）
- `0` 语义：非法，不是无限
