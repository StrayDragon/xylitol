---
depends_on:
  - c3268-fix-turn-model-review-followups
title: "Tag goal continuation guidance with source marker"
needs_specs_change: true
branch: sdd/c3269-update-goal-continuation-tag
base_branch: main
base_sha: e0a6fd20f222c169f4eb8df4f314b53195db17b8
---

# goal 续跑指导加来源标记（c3269）

> 2026-11-12 裁决：选 B——角色维持 user（DSH / Codex / Claude Code 三家实证先例），消息正文包来源标记；裸 user 比三家少的正是标记。依赖 c3268（同文件 react/mod.rs，串行）。

## Why

goal 续跑指导（"目标尚未达成，继续推进\n{guidance}"）现为**裸 user 消息**（r3301 明文，b63532b 落地）。调研实证（research/harness-evidence.md）：三家 harness 全部用 user 角色驱动续轮，但**全部带来源标记**——DSH/Claude 用 user 轮内 `<system-reminder>` 标签，Codex 用 ContextualUserFragment 元数据。裸 user 的代价：transcript 真伪难辨（审计 / 回放 / 摘要失真）；TUI 无法区分渲染。

## What Changes

- 续跑指导正文包 `<system-reminder>` 开闭标签（与 c3248 context_reminder **同一约定**，词表统一）：`<system-reminder>目标尚未达成，继续推进。\n{guidance}</system-reminder>`。
- 投影给模型时**保留标签**（业界惯例：标签即给模型的上下文信号，兼作来源提示）。
- TUI / print 渲染：检测 user 行内的该标签 → 以「系统消息」样式渲染（app-tui 固定词表既有词），不再当真人输入显示。
- spec r3301 补措辞：续跑指导须作为 user 消息并携带来源标记；端侧须按系统消息呈现。

## Capabilities

- agent-goal（r3301 措辞）；app-tui-transcript / app-tui-fixed-zone（系统消息渲染）；cli-print。

## Impact

- 范围：crates/xylitol-runtime/src/agent/runtime/react/mod.rs（goal enqueue 处）、app-tui bridge/print 渲染、两处 spec。无 breaking（追加标签，模型侧为纯文本变化）。

## 决策记录（2026-11-12，全部已决）

1. （已决）角色维持 user（三家先例；改 system/developer 无先例且伤缓存）。
2. （已决）标记用 <system-reminder> 与 c3248 同一约定（词表统一，端侧一套检测逻辑）。
3. （已决）投影保留标签；端侧按「系统消息」固定词渲染。
4. （已决）depends_on c3268（react/mod.rs 同文件串行）。
