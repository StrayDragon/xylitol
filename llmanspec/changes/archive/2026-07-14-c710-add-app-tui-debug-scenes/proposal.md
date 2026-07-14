---
change_id: c710-add-app-tui-debug-scenes
title: "产品 /debug:<scene>：可复现手测场景（Fake + 固定 session）"
status: full
priority: 710
depends_on: ["c685-update-app-tui-session-tree-slot-help"]
author: agent
track: A
---

# c710-add-app-tui-debug-scenes

## Why

手测 Fake / 清 session 后常卡在「Model fake not found」或空树；E2E 有隔离 config，日常 `cargo run -- --trust --tui` 没有一键灌场景。需要 debug 入口装载可复现 fixture，减少对真云与脏 HOME 的依赖。

## Purpose

idle 下 `/debug:<scene>`（及 `/debug <scene>`）装载命名场景：新建隔离 `debug-…` session、写入最少对话树、切换过去并重建 transcript。无参 `/debug` 列出场景。不改生产默认 session 行为；非 Settings UI。

## What Changes

1. `commands`：解析 `/debug` / `/debug:tree-branch` / `/debug tree-labeled` → `PendingSlash::DebugScene`。
2. `Driver::load_debug_scene`：创建隔离 session、种子消息（及 labeled）、switch；返回 entries。
3. `effects` + host：重建 scrollback；系统提示场景名；若 catalog 含 `fake` 则顺带选中。
4. harness：slash 解析 + ScriptedDriver 装载回调。
5. `AGENTS.md` 人类路径改为优先 `/debug:…`。

## Capabilities

- `app-tui-commands`（atm5）
- `app-tui-session-tree`（ast13 场景指针：debug 装载后双 Esc 可见树）

## Out of scope

- 真 LLM debug；热注册 fake 进 registry（无 catalog 时只种子树 + 提示）；fixture 编辑器；改 protocol::Command

## Ethics

- risk_level: low
- prohibited_actions: 默认真扣费 API；覆写用户当前 session 内容（MUST 新建 debug-* session）
- required_evidence: harness + `--strict`

## Depends

- **c685**（已归档）；与 **c705** 互补（手测入口 vs E2E 闸）

## 验证

- Agent：`cargo test --lib -- parse_slash` / harness debug；`llman sdd validate c710-… --strict`
- 人类：`/debug:tree-branch` → 双 Esc → Search/Help；`/debug:tree-labeled` → Ctrl+L labeled-only
