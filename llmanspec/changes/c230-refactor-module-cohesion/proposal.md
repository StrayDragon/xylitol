---
id: c230-refactor-module-cohesion
title: "拆分 god module 并统一 compaction 配置 — 提升内聚性"
depends_on: [c225-refactor-domain-layering]
---

## Why

架构审计发现两个 **god module** 与一处 **配置多源**,严重拖累可维护性:

| 问题 | 现状 | 影响 |
|---|---|---|
| `agent/session.rs` | 1389 行,混合 12 类职责(模型注册/工具注册/持久化/事件订阅/模板/命令分发/压缩编排/skill 激活/bash 执行/sandbox/导入导出) | 单点修改风险高、阅读困难 |
| `infra/session/compaction.rs` | 1519 行,混合 7 类职责(token 估算/切点检测/文件追踪/LLM 摘要/分支摘要/消息转换) | 同上 |
| compaction 配置三源 | `config/types.rs::CompactionConfig` + `session/config.rs::CompactionConfig`(同名第二个)+ `session/compaction.rs::CompactionSettings`(运行时实际使用) | source-of-truth 模糊,bug 温床 |

`AgentSession` 几乎是整个 agent 的 facade,违反单一职责;compaction 单文件 1500+ 行难以独立测试与演进。

**前置**:依赖 c225 完成的 `core/` 层(消息/模型类型已下沉),使拆分在正确的分层结构上进行,避免在旧结构里拆出新的反向依赖。

## What Changes

1. **分解 `agent/session.rs`**:抽取聚焦的协作组件(`ModelManager`、`ToolManager`、`CompactionOrchestrator`、`SkillManager` 等);`AgentSession` 退化为组合这些组件的**薄 facade**,保留既有事件流与 public API。
2. **拆分 `infra/session/compaction.rs`**:按职责切分为 `token_estimator` / `cut_detector` / `file_ops_tracker` / `llm_summarizer` / `branch_summarizer` / `message_converter` 等模块,各自可独立测试。
3. **统一 compaction 配置**:消除重复的 `CompactionConfig`(删除 `session/config.rs` 中的同名定义);明确 `AppConfig.compaction`(YAML 加载期面)→ 运行时 `CompactionSettings` 的**单一映射**,并文档化。

## Capabilities

- `agent-session`:新增内聚性规约(as30/as31)。
- `compaction`:新增模块拆分与配置单一源规约(c13/c14)。
- `runtime-config`:新增 compaction 配置映射规约(rc15)。

## Impact

- **规模**:大型内部重构,触及 session/compaction 相关调用点。
- **行为**:零行为变更;纯结构调整 + 配置类型合并。
- **安全网**:454 lib 测试 + 83 BDD 场景;facade 保留确保 public API 不变。
- **风险**:低于 c225(无分层模型变更,仅在既有层内重组)。facade 模式保证调用方无感。
- **不做**:不引入新功能、不改 ReAct 循环语义、不动 session 持久化格式。
