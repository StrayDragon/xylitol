---
change_id: c1150-add-app-tui-thinking-level
title: "产品 TUI：thinking level 边框 + footer 回退显示"
status: draft
priority: 1150
apply_band: P3-feature
depends_on:
  - c1140-add-package-tui-thinking-level-border
  - c1145-update-runtime-model-thinking-levels
author: agent
track: R
wave: thinking
domain: app-tui
ethics:
  risk_level: low
  prohibited_actions:
    - 无配置模型上强行显示伪造 level
    - cycle 时向 transcript 刷 thinking-border 系统块
    - 产品默认加 /thinking-level slash
    - 用 ThinkingBorderLevel::cycle_next 冒充模型支持集 cycle
  required_evidence:
    - harness：Shift+Tab cycle 后 footer 与边框一致
    - cycle 后 scrollback 无 thinking-border 系统行
    - busy 下仍可 cycle 且无拒绝系统块
  escalation_policy: busy 策略已钉「idle+busy 均可」；勿改回 idle-only 除非 PI 刻意差异
---

# c1150-add-app-tui-thinking-level

## Why

包能力（c1140）+ 模型配置（c1145）就绪后，产品面接线。不做 `/settings`；用 **Shift+Tab** cycle（对齐 pi `app.thinking.cycle`）；边框色 + footer 旁 level 标签双通道显示。

## Purpose

产品 TUI：显示并切换 thinking level（来源 c1145 / `Driver`）；优先编辑器边框色（c1140 `apply_thinking_border`）；footer 同步 `• thinking off` / `• xhigh`…；切换经 Driver/protocol；**idle 与 busy 均可 cycle**；**静默**（无 transcript 刷屏）。

**UX 静默（对齐 pi，与 agent_demo 区分）**：

| 面 | Shift+Tab / cycle 反馈 |
|---|---|
| **`agent_demo`** | 可为验证「上行」：transcript 系统行（如 `thinking-border → medium`）+ status |
| **产品 `src/app/tui`** | **MUST 静默**：只改编辑器 thinking 边框色 + footer 旁 level 文案；**MUST NOT** 向 live scrollback 追加 `thinking-border → …`；**MUST NOT** 加产品 `/thinking-level` slash |

## What Changes

- `Driver::cycle_thinking_level`（或等价）暴露 `ModelManager::cycle_thinking_level`；产品经 seam，不 reach manager
- host/footer/editor：`ThinkingLevel` → `ThinkingBorderLevel` + footer `• {label}`（off → `thinking off`）
- 快捷键：`app.thinking.cycle` 默认 **Shift+Tab**（可配置；不抄 demo 其它 Ctrl 和弦）
- bash 边框覆盖 thinking；退出 `!` 前缀后恢复 thinking 边框（改 ati15）
- 主题切换后重涂 thinking 边框；模型切换后从 `Driver::thinking_level` 重同步
- harness：边框/footer 一致；cycle 后 transcript 无 thinking-border 系统行；busy 可 cycle
- delta：`app-tui-chrome` · `app-tui-input` · `app-tui-host`
- 同步 `design/footer.md` · `design/keybindings.md`

## Capabilities

- `app-tui-chrome`（modify）
- `app-tui-input`（modify）
- `app-tui-host`（modify）

## Out of scope

- `/settings`
- `/thinking-level` 产品 slash
- 改 agent_demo 上行反馈（demo 可继续嘈杂）
- 改 PI_DELTAS 刻意差异以外行为
- 发射/消费 `XyEvent::ThinkingLevelChanged`（本变更以同步读 `thinking_level()` 为准；事件发射可后置）

## Impact

- 用户可在产品 TUI 用 Shift+Tab 切换 thinking level，边框与 footer 即时反馈，无系统块噪音

## Depends

- c1140 · c1145（已归档）
