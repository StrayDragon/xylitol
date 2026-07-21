---
change_id: c1450-add-config-vars-home
title: 配置 YAML 模板支持 {{ vars.home }}
status: in-progress
priority: 1450
depends_on: []
author: agent
branch: feat/c1450-add-config-vars-home
base_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
checkpointed: true
checkpoint_sha: c8fa127776ea9cd8bb8ddcc23dfd7c9661721249
---

# c1450-add-config-vars-home

## Why

1. 可分享的 `.xylitol/config.yaml`（可进 git）里 MCP `command` 等字段常写死本机绝对路径（如 `/home/<user>/…`），造成用户名外漏与跨机不可移植。
2. 已有 `{{ env.* }}` / `{{ secret.* }}`；需要一等、窄范围的路径变量，避免鼓励把 `HOME` 环境依赖或更多路径面写进共享配置。
3. **刻意只暴露 `home`**：不提供 project/cwd 等，降低误把敏感路径语义写进可分享配置的安全面。

## What Changes

- 配置 YAML minijinja 渲染增加命名空间 `vars`，**仅**键 `home`（解析为用户 home 目录绝对路径，`dirs::home_dir`）
- 不可解析 home → 与现有一致：strict 模板错误，配置加载失败
- `vars` 下其它键（如 `project`）→ undefined，strict 失败（**MUST NOT** 静默支持）
- 文档 / example / 仓库 `.xylitol/config.yaml` 中 lspz 示例改为 `"{{ vars.home }}/.cargo/bin/lspz"`
- 单测覆盖渲染成功与未知键失败；**不**为静态插值单独扩 BDD step（对齐 rc22 风格）

## Out of scope

- `vars.project` / `vars.cwd` / `vars.config_*` 等其它路径变量
- 改 `secret.*` / `env.*` 语义
- MCP PATH 解析增强、自动发现 cargo bin
- 把本机 MCP 迁出项目配置（产品建议可写全局 config，非本 change）

## Capabilities

- `runtime-config`（新增 rc23）

## Ethics

- risk_level: low
- prohibited_actions: 新增 `vars` 下除 `home` 外的路径键；把用户绝对路径写回可分享示例
- required_evidence: `render_config_template` 单测绿；`{{ vars.home }}` 可渲染；未知 `vars.*` strict 失败；相关 docs/example 已更新
- escalation_policy: 若需暴露 project/cwd → STOP，另开 change 与安全评审
