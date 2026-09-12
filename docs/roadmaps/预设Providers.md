# 预设 Providers

> 更多开箱可辨的厂商/网关预设：先抽主要者；被迫走 TS 集成的（如 Cursor SDK）单独叙事。
> 现状对齐：2026-07-17。

## 用户怎么碰到

- 想「选一个名字就能用」，而不是手写兼容端细节；
- 预设应声明自己是哪类兼容 API，进入既有 OpenAI-like / Anthropic-like 故事；
- 少数能力只能通过 TS/SDK 集成——用户应感到这是**协同或旁路**，不是第二套 xylitol 内核。

## 优先清单（产品意向）

| 名称 | 角色（产品） | 备注 |
|---|---|---|
| **ZModel（GLM）** | 常用国产/智谱系入口预设 | 走兼容 API 开闭 |
| **OpenCode Zen** | 网关/聚合类预设 | 声明兼容族 |
| **ClinePass** | 网关/代理类预设 | 声明兼容族 |
| **CommandCode** | 网关/代理类预设 | 声明兼容族 |
| **Cursor SDK** | TS 侧 agent 协同 | 与 [Cloud-Agent与Web控制台.md](./Cloud-Agent与Web控制台.md) 协同；**可独立于 xylitol 使用** |

清单可增删；本文件记**方向**，具体字段以当版配置心智为准。

## 产品规则

| MUST | 禁止 |
|---|---|
| 预设只扩展接入与默认档案，不改会话/工具主线故事 | 为每个网关分叉 ReAct 语义 |
| 预设标明兼容族与能力边界（含模态/工具） | 暗示未支持的能力「都有」 |
| 遵守 [../architecture/多厂商模型.md](../architecture/多厂商模型.md) 已落地的协议族 / `api` / `compat` 现行 MUST（本文不复述） | 偷偷引入第四类传输当默认；把 Completions 当「遗留/将删」叙事 |
| TS/SDK 集成放在 Web/协同面 | 让 Rust 内核依赖 Cursor 才能对话 |

## BDD 意图示例

**场景：选预设即用**
Given 用户选择某一已维护预设并配置密钥
When 开始对话
Then 行为与开箱兼容厂商同构（在该预设声明的能力内）

**场景：Cursor 协同可选**
Given 用户未配置 Cursor SDK
When 仅使用 xylitol TUI/Print 对话
Then 无 Cursor 依赖；配置后可按协同路线调度

## 分阶段

| 阶段 | 用户可感知结果 |
|---|---|
| M1 档案与文档 | 主要预设可配置、可发现 |
| M2 验证过的默认 | 各预设有「已知可用」基线说明 |
| M3 与计量/检视 | 词表策略、观测出口对网关友好；排障时能对照「线路 vs 理解」（见 [OTEL与Langfuse观测.md](./OTEL与Langfuse观测.md)） |
| M4 Cursor SDK | Web 协同面落地（见 Cloud 路线图） |

## 依赖

- 开闭扩展心智：[../architecture/多厂商模型.md](../architecture/多厂商模型.md)、[../architecture/产品分层总览.md](../architecture/产品分层总览.md)
- 与 Tokenizer、观测出口、Web 可交错，不互相硬阻塞
- 厂商/网关排障：[OTEL与Langfuse观测.md](./OTEL与Langfuse观测.md)；对照底座见 [../architecture/进程内观测.md](../architecture/进程内观测.md)

## 支线与方向

| 支线 | 意向 |
|---|---|
| **Cache 能力声明格** | 各预设是否透出 prompt cache / TTL / 最小前缀（挂缓存 roadmap） |
| **兼容回归 Dataset** | 新预设合并前跑小回归集（挂 [Agent-Eval与回归基准.md](./Agent-Eval与回归基准.md)） |
| **国产/网关词表策略** | 计量与 tokenizer 映射说明，避免启发式假「官方」 |
| **预设发现 UX** | 「常用名」列表与文档同源，避免只活在 yaml 注释 |
| **失效预设退役叙事** | 废弃预设诚实提示迁移路径，不静默改写 |

## 相关

- [上下文缓存与极致压缩.md](./上下文缓存与极致压缩.md)
- [Agent-Eval与回归基准.md](./Agent-Eval与回归基准.md)
- 总索引：[README.md](./README.md)
