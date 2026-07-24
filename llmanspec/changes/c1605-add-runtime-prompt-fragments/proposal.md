---
change_id: c1605-add-runtime-prompt-fragments
title: 运行时能力 ↔ 系统提示片段自动注入（少配置面）
status: purpose-draft
priority: 1605
depends_on: []
author: agent
---

# c1605-add-runtime-prompt-fragments

## Discussion context

见 [`../DEFERRED-responses-tool-batch-CONTEXT.md`](../DEFERRED-responses-tool-batch-CONTEXT.md)。

临时态：`.xylitol/APPEND_SYSTEM.md` 已放实验 3 多-tool 片段；本 change 要把「策略→片段」做成机制，去掉手贴 APPEND。

## Why

开箱行为要靠 **默认能力 + 匹配的系统提示**，而不是让用户同时拧 `tool_batch.mode` 又手写 APPEND_SYSTEM。

后续 TUI「运行时切换 tool 运行策略」时，若只改调度器、不改提示，模型仍会单 tool/turn——并行 ROI 接近 0。需要：

```text
能力/策略状态（配置或会话覆盖）
        │
        ▼
  prompt fragment registry（内置、可测）
        │
        ▼
  build_system_prompt / 下一波次重绑
```

与候补 [运行时即时设置 · 能力覆盖盘](../../../docs/roadmaps/运行时即时设置.md) 同轴：覆盖再生效波次 = 提示重绑波次。

## Product intent

| MUST | 禁止 |
|---|---|
| 内置策略变更自动带上/撤下对应片段 | 为每个策略再要用户贴一段 system |
| 片段短、可测、可关（debug） | 把整份 pi 提示复制成不可维护墙 |
| 与 `SYSTEM.md` / `APPEND_SYSTEM.md` 可叠加且顺序稳定 | 静默覆盖用户 SYSTEM.md 全文 |
| 会话覆盖切换策略 → 下一 turn 提示已变 | 暗示已切换但模型仍吃旧提示 |

## Decisions（意向）

1. **Fragment 源**：代码内置表（`tool_batch=barrier_parallel` → multi-tool 同消息指引；未来 LSP on → 短指引…）；非用户 YAML 碎文件森林。
2. **注入点**：`build_system_prompt` 的 guidelines / 专用 `<runtime_policy>` 段；用户 `append_system_prompt` 仍在后。
3. **生效波次**：与 Session 能力快照一致（对齐 NextTurn / 覆盖盘）；run 开始冻结。
4. **可观测**：可选 debug：记录本 turn 启用的 fragment id 列表（勿默认把全文打进 Langfuse）。
5. **少配置**：不新增「prompt_fragments:」用户配置节作为主路径；高级覆盖后置。

## 与 c1610 关系

本 change = **机制**。`c1610` = 选用哪些默认策略 + 默认片段内容（含 tool 批开箱）。

## What Changes（promote 后）

- fragment registry + 单测（启停片段字符串稳定）
- Session/bootstrap 把当前 `tool_batch.mode`（及未来覆盖）映射到 fragment ids
- roadmap：运行时即时设置 M1 注明「策略切换触发片段」

## Non-Goals

- 完整覆盖盘 UI（仍 roadmap）
- 改 ReAct 执行语义
- 用户自定义 Jinja 模板市场

## Status

**purpose-draft** — 可与 `c1610` 同波设计；建议先落地机制再填默认文案。

## Ethics

- risk_level: low–medium（提示影响模型行为；须可关、可测）
- required_evidence: 切换策略前后 system 中 fragment 有无的单测；人工 A/B 见实验 3
