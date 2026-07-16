---
change_id: c1105-add-app-tui-trust-slash
title: "产品 slash：/trust 保存项目信任决策"
status: draft
priority: 1105
apply_band: P3-feature
depends_on: []
author: agent
track: R
wave: slash-simple
domain: app-tui
ethics:
  risk_level: medium
  prohibited_actions:
    - 未确认就 always-trust 全局
    - 绕过 Trust 闸静默加载项目扩展
    - /trust 成功后自动调用昂贵 /reload
  required_evidence:
    - trust.json 写盘可测
    - busy 拒绝
    - 成功提示含 /reload 或重启
  escalation_policy: 已钉「写盘 + 提示重载」；Choice 槽仍冻结勿扩
---

# c1105-add-app-tui-trust-slash

## Why

pi `/trust` 可在交互内写入信任决策。xylitol Trust 在 CLI gate / ChoicePrompt 启动路径已有；产品 idle slash 表缺 `/trust`。

## Purpose

产品 TUI **idle** 解析 `/trust`：

1. 经 Driver 缝将信任决策写入 Trust 持久 store（cwd / parent / deny 子命令，见 design）
2. **busy** MUST 拒绝且 MUST NOT 写盘
3. 成功后系统块提示：**本会话不自动重载**；需 `/reload` 或重启后项目资源才按新信任生效（reload 成本高，故意不内联）
4. MUST NOT 解冻 Plate/Settings/Choice 活板；MUST NOT reach `infra` from `app/tui`

## What Changes

- `commands` + product slash catalog：`/trust`
- `Driver::persist_project_trust`（或等价）+ `InProcessDriver` → `TrustManager`
- `effects/slash`：编排写盘 + 提示文案
- harness：写盘可观测（scripted）/ busy 拒绝 / 不调用 `reload_runtime`
- delta：`app-tui-commands` · `app-tui-trust`

## Capabilities

- `app-tui-commands`（modify）
- `app-tui-trust`（modify）

## Out of scope

- 解冻 ChoicePrompt 活板做交互选择器（启动 gate 已有）
- `/trust` 后自动 `/reload`
- 工具 permission popup

## Impact

- 会话内可补写信任决策，而不付同步 reload 成本
