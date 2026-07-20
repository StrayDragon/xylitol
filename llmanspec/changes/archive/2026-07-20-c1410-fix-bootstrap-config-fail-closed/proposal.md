---
change_id: c1410-fix-bootstrap-config-fail-closed
title: 配置加载失败硬失败；禁止 env 兜底伪装成 gpt-4o
status: full
priority: 1410
depends_on: []
author: agent
track: B
wave: config-ergonomics
domain: cli-entry
apply_band: fail-closed
branch: feat/c1410-fix-bootstrap-config-fail-closed
base_sha: fd9a5c9e2fb59f2b8de5ed3a144b3892b133c437
checkpointed: true
checkpoint_sha: fd9a5c9e2fb59f2b8de5ed3a144b3892b133c437
---

# c1410-fix-bootstrap-config-fail-closed

## Why

`bootstrap` 在 `load_app_config` 失败时只推 `ConfigLoadFailed` Warning，再静默用 `OPENAI_API_KEY` 注册并选中 `gpt-4o`。用户本机 `.xylitol/config.yaml` 若因注释里的 `{{ secret.KEY }}` 模板炸了，TUI 仍进入且 footer 显示厂商模型名——与真实配置无关，极易误导。

既有 **ce2**（配置存在但零模型时禁止静默回退占位默认）未被落实到「加载失败」路径。

## Purpose（已钉）

1. **配置加载失败硬失败**：YAML/模板/IO/`LoadError` → `BootstrapError`（或等价），**全表面**（TUI / print / `--list-models` / server 经同一 bootstrap）MUST 退出并打印可读错误；MUST NOT Warning 后继续装配。
2. **落实 ce2**：配置已加载但可注册模型为 0 → MUST 硬失败指向配置；MUST NOT 再走 env 填 `gpt-4o`。
3. **禁止伪装选中**：仅有 provider API key、无配置模型且无 `--model` 时，MUST NOT 自动将 `gpt-4o` / `claude-…` 等厂商默认 ID 设为当前选中模型；未选中态产品展示 MUST 为 `NOT-SET`（或等价明确占位），TUI MUST 继续由既有 `NoModelSelected` 拦截进面。
4. **m3 保留**：`default_model_id_for_provider` 仍可返回真实可用 ID，供显式解析/回退辅助；与「bootstrap 不得静默选中」分离。
5. **仓内修复**：修正 `.xylitol/config.yaml` 注释中的字面 `{{ secret.KEY }}`，避免文档示例触发模板错误。

## What Changes

- `src/app/core/bootstrap.rs`：`ConfigLoadFailed` → `BootstrapError`；零模型 + 有配置 → 硬失败；收紧/移除「空 registry 时 env 自动 register+select gpt-4o」路径
- `src/app/cli/mod.rs`：错误文案；去掉「falling back to env vars」Warning 主路径
- TUI/Driver 未选中展示：`NOT-SET`（替换易被忽略的 `—`，若该处仍为未选中）
- live `cli-entry`：强化 ce2 + 新 req；`runtime-model-registry`：补「bootstrap 不得静默选中」边界（不改 m3 真实 ID 语义）
- 单测 / 既有 harness；必要时补 BDD 场景
- `.xylitol/config.yaml` 注释转义

## Out of scope

- 改 minijinja 使 YAML 注释免渲染（可另开；本 change 以 fail-closed + 修注释为主）
- 迁移用户 `config.local.yaml` 内容
- 改变 provider 官方默认 ID 表（m3）

## Capabilities

- `cli-entry`（modify）
- `runtime-model-registry`（modify，边界澄清）

## Ethics

- risk_level: medium（破坏「仅 API key 即可默默进 TUI」的宽松路径）
- prohibited_actions: 保留 ConfigLoadFailed 后静默进 TUI；用厂商模型名冒充未配置状态
- required_evidence: bootstrap/CLI 测证明加载失败硬退出；无配置选中时展示 `NOT-SET`；ce2 零模型硬失败
- escalation_policy: 无——用户已确认完整 SDD + fail-closed
