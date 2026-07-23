---
change_id: c1605-add-runtime-prompt-fragments
title: 运行时能力 ↔ 系统提示片段自动注入（少配置面）
status: purpose-draft
priority: 1605
depends_on: []
author: agent
---

# c1605-add-runtime-prompt-fragments

## Discussion context（2026-07-24）

- 批并行 ROI 在「模型单 tool/turn」时≈0；单靠调度器不够，需要匹配的系统提示。
- 临时态：`.xylitol/APPEND_SYSTEM.md` 已放实验 3 多-tool 片段；本 change 把「策略→片段」做成机制，去掉手贴 APPEND。
- 与候补「运行时即时设置 / 能力覆盖盘」同轴：切换 tool 策略 → 下一波次自动换片段。
- 默认片段内容与开箱 `barrier_parallel` 见 `c1610`。

## Why

开箱行为 = 默认能力 + 匹配提示，而不是用户同时拧配置又手写 SYSTEM。

## Product intent

| MUST | 禁止 |
|---|---|
| 内置策略变更自动带上/撤下对应片段 | 为每个策略再要用户贴一段 system |
| 片段短、可测、可关 | 不可维护的提示墙 |
| 与 SYSTEM.md / APPEND_SYSTEM.md 可叠加且顺序稳定 | 静默覆盖用户 SYSTEM.md 全文 |
| 会话覆盖切换 → 下一 turn 提示已变 | 暗示已切换但模型仍吃旧提示 |

## Decisions（意向）

1. Fragment 源：代码内置表（非用户 YAML 碎文件森林）。
2. 注入点：`build_system_prompt` 专用段；用户 append 仍在后。
3. 生效波次：与 Session 能力快照一致。
4. 不新增「prompt_fragments:」用户配置主路径。

## Non-Goals

完整覆盖盘 UI；改 ReAct 执行语义；用户 Jinja 模板市场。

## Status

**purpose-draft** — 建议先落地机制再填默认文案（`c1610`）。

## Ethics

- risk_level: low–medium
- required_evidence: 启停片段单测；实验 3 命中率
