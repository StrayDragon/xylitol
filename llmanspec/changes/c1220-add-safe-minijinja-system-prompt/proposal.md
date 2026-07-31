---
depends_on:
  - c1218-remove-slash-prompt-templates
---

# 安全 minijinja 默认 system 组装

> 由历史 draft `c1220-add-jinja2-prompt-templates` rename；深挖决策 D1–D13 见下。前置 **c1218** 已归档。

## Why

默认 system 组装今日是 `SystemPromptOpts` + Rust 字符串拼接：文案难外置；`date` 非确定；与「安全声明式模板」目标冲突。live **pt5** 禁止 jinja，须改写为沙箱 minijinja MUST。用户 A/B 继续靠 workspace / SYSTEM.md（不多 profile）。

## Decisions

| # | 决策 |
|---|---|
| D1 | 安全 minijinja（agent 自建沙箱 Env，↛ infra） |
| D2/D7 | slash prompts 已由 c1218 移除 |
| D4 | 单一默认入口；不多 prompt_profile |
| D6 | SYSTEM/APPEND 纯文本，不跑 Jinja |
| D8 | 本 id：`c1220-add-safe-minijinja-system-prompt` |
| D9 | 保留 `build_system_prompt`；可注入 date |
| D10 | 入口 + 少量预注册 partials；反散落；人类可编 j2 |
| D11 | ctx：date/cwd/tools/guidelines/skills/runtime_policy/mcp_discover；禁 env/secret |
| D12 | pt5 MUST 沙箱 minijinja |
| D13 | 单测为主 + 改写 pt5 场景 |

## What Changes

- 默认组装改为嵌入 j2 + 沙箱 render；`build_system_prompt` 门面保留；opts 可注入 date（缺省 Utc::now）
- SYSTEM/APPEND/context 纯文本拼接顺序保持产品语义；skills XML / guidelines / runtime_policy / builtins-only（pt11）语义保留
- 改写 `agent-prompt` pt5：废 `no-jinja-dep`；新安全渲染 MUST
- 少量 partials（`{% include %}` 仅预注册名）；复杂过滤留 Rust

## Capabilities

- `agent-prompt`（主）

## Impact

- 热路径 system 字符串；须既有 BDD/单测回归
- 依赖已归档 c1218

## Out of scope

- 多 prompt_profile；eval profile YAML；用户文件 Jinja；CLI dump；SWE/Harbor 管线
