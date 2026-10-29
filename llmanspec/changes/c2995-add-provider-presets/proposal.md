---
depends_on: []
title: 为常用兼容端提供配置预设
needs_specs_change: true
---

# 为常用兼容端提供配置预设

> 来源：GitHub issue #20（预设 Providers）M1。
> 收口约定：实现落地时 commit / PR 文案须含 `fixed #20`，由默认分支合并自动关闭 issue；本规划 commit 只写 `refs #20`，避免提前关单。

## Why

- 用户要「选一个名字就能用」，但当前必须在 `models.<name>` 手写 `provider / model / base_url / api / compat / thinking_levels`。
- 这些字段对同一网关高度重复，且 `api` / `compat` 选错会直接导致请求失败。
- 预设只应扩展接入与默认档案，不得为每个网关分叉 ReAct 语义。

## What Changes

- 新增内置预设目录（编译期常量）：ZModel（GLM）、OpenCode Zen、ClinePass、CommandCode；每个预设声明兼容族、`api`、`compat`、`base_url`、推荐模型与 thinking 档位。
- `ModelEntry` 增加可选 `preset: Option<String>`；解析时预设提供默认值，显式字段覆盖预设；未知预设给出可读诊断。
- 预设不携带密钥；密钥仍经 `{{ secret.KEY }}` 或环境注入。
- `configs/example.yaml` 增加预设示例；文档列出每个预设的兼容族、能力边界与「已知可用」基线。
- 更新 `runtime-config` / `infra-provider` 规则与场景；补解析优先级与未知预设测试。

## Capabilities

- `runtime-config`：模型档案解析。
- `infra-provider`：预设到 adapter 选择的映射。

## Impact

- `crates/xylitol-plugin-sdk/src/model_entry.rs`：`preset` 字段。
- `plugins/xylitol-official-plugin-config/src/`：预设目录与解析。
- `configs/example.yaml` / `scripts/gen_config_example.py`：示例。
- `llmanspec/specs/runtime-config/`、`infra-provider/`。

## Out of Scope

- 不做运行时预设市场 / 下载。
- 不做 Cursor SDK 的 TS 协同端（issue M4，搁置）。
- 不为预设分叉 ReAct、工具表或会话故事。
- 不把预设写成第二套配置真源。

## Open Questions

- 预设名用短名（`zmodel`）还是厂商全名（`z-ai`）？倾向短名 + 文档别名。
- 预设是否允许覆盖 `thinking_levels` 默认值？倾向允许，但必须显式写出，不做隐式猜测。
