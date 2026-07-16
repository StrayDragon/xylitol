---
change_id: c1150-add-app-tui-thinking-level
title: "产品 TUI：thinking level 边框 + footer 回退显示"
status: purpose-draft
priority: 1150
apply_band: P3-feature
depends_on:
  - c1140-add-package-tui-thinking-level-border
  - c1145-update-runtime-model-thinking-levels
author: agent
track: R
wave: thinking
domain: app-tui
---

# c1150-add-app-tui-thinking-level

## Why

包能力（c1140）+ 模型配置（c1145）就绪后，产品面接线。不做 `/settings`；用 **Shift+Tab** cycle（对齐 pi `app.thinking.cycle`）；**可回退**到 footer 旁模型区显示当前 level（`• thinking off` / `• xhigh`…）。

## Purpose

产品 TUI：显示并切换 thinking level（来源 c1145）；优先编辑器边框色（c1140）；边框未就绪或窄宽时 footer 显示 level；切换经 Driver/protocol，busy 策略升格钉死。

**UX 静默（对齐 pi，与 agent_demo 区分）**：

| 面 | Shift+Tab / cycle 反馈 |
|---|---|
| **`agent_demo`** | 可为验证「上行」：transcript 系统行（如 `thinking-border → medium`）+ status，便于目视/harness |
| **产品 `src/app/tui`** | **MUST 静默**：只改编辑器 thinking 边框色 + footer 旁 level 文案（如 `• thinking off` / `• xhigh`）；**MUST NOT** 向 live scrollback / transcript 追加 `thinking-border → …` 类系统块；**MUST NOT** 加产品 `/thinking-level` slash |

升格 full 时把上表钉进 delta（chrome/host）。

## What Changes（升格 full 时）

- host/footer/editor 接线：`ThinkingLevel` → `ThinkingBorderLevel` + footer `• {level}`
- 快捷键：默认 **Shift+Tab** cycle（可配置；不抄 demo 其它 Ctrl 和弦为产品默认）
- busy 策略；无 transcript 刷屏
- harness：边框/footer 一致；cycle 后 transcript 无 thinking-border 系统行
- delta：`app-tui-chrome` · `app-tui-input` · `app-tui-host`

## Capabilities

- `app-tui-chrome`（modify）
- `app-tui-host`（modify）

## Out of scope

- `/settings`
- `/thinking-level` 产品 slash
- 改 PI_DELTAS 刻意差异以外行为
- 改 agent_demo 上行反馈（demo 可继续嘈杂）

## Ethics

- risk_level: low
- prohibited_actions: 无配置模型上强行显示伪造 level；cycle 时向 transcript 刷系统块
- required_evidence: 切换后 footer/边框一致且无 transcript 刷屏；协议事件可测
- escalation_policy: 仅 footer 分期交付可接受，须在 tasks 标明

## Depends

- c1140 · c1145
