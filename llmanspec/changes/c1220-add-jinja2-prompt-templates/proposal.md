---
depends_on: []
---

# c1220 — Prompt 管理重构 / system 抽出（延后）

> **purpose-draft / deferred**。目录名仍为 `c1220-add-jinja2-prompt-templates`（历史 draft id）；**主题已改**：主目标是 **prompt 管理逻辑重构重写** + **抽出 system prompt 组装**，便于后续 eval / 调优。模板引擎（minijinja 等）只是可选实现手段，**其次**。
>
> 升格时可用 `llman-sdd-propose`；若改 id/目录名，propose 时一并整理。

## Why

今日 system prompt 组装偏散：`SystemPromptOpts` 多字段 + Rust 字符串拼接 + tools 散文列表混在运行时路径里。结果：

- 改一句文案 / 调一段结构要翻代码；
- **eval / 调优**难以对「同一套 prompt 产物」做离线对比（缺稳定抽出与可替换边界）；
- 用户/项目级定制与组合缺少清晰模块边界。

需要一次 **管理逻辑重构**：把「组装契约、块组合、默认文案」从 Agent 热路径里摘清楚，而不是先绑死某个模板引擎。

## Purpose（暂缓 · 主次）

| 优先 | 内容 |
|---|---|
| **主** | 重构 `agent/prompt`（或等价）管理：明确块、顺序、覆盖规则；**抽出**可单测 / 可 dump 的 system 组装入口，供 eval 与调优 |
| **次** | 若组装需要声明式模板，再选 **minijinja**（或同类）做安全渲染；嵌入默认 + 用户模板；可组合 include |
| **非主** | 「为上 Jinja 而上 Jinja」——无抽出与 eval 缝则不做引擎迁移 |

升格前钉：

1. 抽出后的 **公共 API**（输入：opts/块；输出：最终 system 字符串 + 可选结构化块清单）。
2. 与 c1210「builtins-only Available tools + MCP discover 一句」如何落在默认块里。
3. 用户覆盖路径与 trust 闸。
4. 若引入模板引擎：安全边界（禁任意代码、路径逃逸、读秘密）。

## What Changes（升格 full 时 · 意向）

- 重写 / 收拢 prompt 组装模块；运行时只依赖抽出的组装缝。
- 提供 dump/fixture 友好的纯函数或端口，便于 eval 管线对比 prompt 文本。
- （可选）minijinja 安全渲染 + 嵌入默认模板 + 用户模板组合。
- specs：`agent-prompt`（管理边界、抽出 MUST、安全 SHOULD）。

## Capabilities

- `agent-prompt`（主）
- 可能触 trust / resource discovery（用户模板加载）

## Impact

- 面广、易回归 → **延后**、单独 propose；不阻塞 c1210。
- 做好后 eval 调优成本应明显下降。

## Out of scope（本草案）

- c1210 解闸 / tools overlay / MCP 进程关停
- 立刻换引擎或改 live specs
