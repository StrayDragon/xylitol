---
depends_on: []
---

# Proposal: Phase 3 Infrastructure Gaps — 补齐模型、资源、工具差距

## Why

C20 Phase 2 完成后，xylitol 核心约完成 **65-70%**。剩余约 30% 涵盖模型管理、资源加载、模板/命令系统、输出缓冲等基础设施。这些是 pi 的「粘合层」— 没有它们，AgentSession 无法独立启动一个完整会话。

与 pi 对比后，识别出 5 个 P1 差距和 5 个 P2 差距。

## What Changes

### P1 — 影响功能完整性

| # | 模块 | pi 参考 | 行数 |
|---|------|---------|------|
| 1 | **ModelResolver** | `model-resolver.ts` (640L) | 模型发现、别名解析、默认值、scoped models |
| 2 | **ModelRegistry 升级** | `model-registry.ts` (400L) | provider 注册、auth 检查、available 列表、OAuth |
| 3 | **ResourceLoader** | `resource-loader.ts` (1025L) | Agents.md/CONTEXT.md 加载、技能路径、主题发现 |
| 4 | **Prompt Templates** | `prompt-templates.ts` (284L) | `/template:name` 文件模板系统、参数替换 |
| 5 | **OutputAccumulator** | `output-accumulator.ts` (222L) | 流式输出缓冲、temp file、增量截断 |

### P2 — 影响稳定性/可用性

| # | 模块 | pi 参考 | 行数 |
|---|------|---------|------|
| 6 | **Slash Commands** | `slash-commands.ts` (41L) | 内置命令表 `BUILTIN_SLASH_COMMANDS` |
| 7 | **Defaults / Diagnostics** | `defaults.ts` + `diagnostics.ts` (65L) | 集中默认值 + 启动诊断 |
| 8 | **SessionCWD** | `session-cwd.ts` (59L) | 会话 CWD 存在性验证 |
| 9 | **PackageManager 检测** | `package-manager.ts` (100L) | npm/pnpm/yarn/bun 检测 |
| 10 | **OutputGuard** | `output-guard.ts` (108L) | stdout/stderr 劫持与恢复（用于 print 模式） |

### 不纳入本次

- **AgentSessionRuntime** (~430L) — 严重依赖 Extensions SDK，延后到 Extensions
- **AgentSessionServices** (~200L) — 同上
- **Auth Guidance** (~30L) — 轻量，整合进 ModelRegistry
- **ResolverConfigValue** (~50L) — 整合进 ModelResolver
- **SourceInfo** — pi 的 metadata 包装器，整合进 ResourceLoader
- **Experimental / Telemetry / Timings** — P3

## Capabilities

| Capability | Action |
|-----------|--------|
| agent-session | add: PromptTemplate expansion, slash command dispatch |
| agent-runtime | add: SessionCWD validation, diagnostics collection |
| model-registry | **new spec**: ModelRegistry + ModelResolver (provider discovery, auth check, default per provider) |
| tool-system | add: OutputAccumulator for streaming tools |
| session-persistence | add: SessionCWD assertion on load |

## Impact

- **新增文件**: `src/agent/registry.rs` (ModelRegistry 升级), `src/agent/resolver.rs` (ModelResolver), `src/infra/resource.rs` (ResourceLoader), `src/agent/templates.rs` (PromptTemplate), `src/agent/commands.rs` (SlashCommands)
- **修改文件**: `src/agent/session.rs` (集成模板/命令), `src/agent/tools/bash.rs` (使用 OutputAccumulator), `src/infra/session/manager.rs` (CWD 验证)
- **向后兼容**: 所有新增模块通过 AgentSession 暴露，不影响现有 API
