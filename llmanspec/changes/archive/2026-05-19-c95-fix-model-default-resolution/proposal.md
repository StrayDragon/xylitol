---
depends_on:
  - c10-add-config
  - c15-add-cli
---

# c95-fix-model-default-resolution

## Why

当前代码存在“写死默认模型”的行为（例如把本机 Pi 的模型 ID 直接作为 `AppConfig` 默认值），这会导致：

- 不同机器/环境下行为不一致（不可复现、不可移植）。
- 违背“可配置优先”的设计原则：运行时应从用户配置/CLI 选项中解析模型，而不是在代码里内置环境相关默认值。

## What changes

- 将 `model.default_model` 视为可选配置项：当用户未提供任何模型配置时，在真正需要构建 LLM 时返回清晰错误，而不是默默使用硬编码默认值。
- 统一默认模型解析优先级（高 → 低）：
  1) `--model <id>`
  2) `agents.profiles.<name>.model`
  3) `execution.model`
  4) `model.default_model`
  5) 都缺失则报错，提示用户如何配置

## Capabilities

- `runtime-config`

## Impact

- 零配置运行（不提供任何 model 配置且不传 `--model`）将从“隐式默认模型”变为“显式报错提示配置模型”。
- 需要更新相关 JSON Schema 与单元测试。
