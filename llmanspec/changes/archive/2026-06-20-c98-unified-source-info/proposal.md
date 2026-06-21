---
depends_on: []
---

# c98-unified-source-info: 统一起源标记类型

## Why
pi 的 `source-info.ts`（40 LOC）定义了统一的 `SourceInfo { path, source, scope, origin, base_dir }`，跨 skills / prompts / themes / extensions 使用。xylitol 目前：
- `infra/skills/loader.rs` 有自己的 `SourceInfo` 结构（source / scope / base_dir）
- `infra/resource/loader.rs` 的 `PromptTemplate` 有 `source_path` 但无 scope/origin
- `agent/commands.rs` 的 `SlashCommandInfo` 有 `source_path` 但无统一类型
- 没有 `create_source_info()` / `create_synthetic_source_info()` 工厂函数

导致资源来源信息不一致，无法追溯一个资源来自哪个 package/scope。

## What Changes
- 在 `src/infra/mod.rs`（或新建 `src/infra/source_info.rs`）定义通用类型：
  - `SourceScope` 枚举（User / Project / Temporary）
  - `SourceOrigin` 枚举（Package / TopLevel）
  - `SourceInfo { path, source, scope, origin, base_dir }`
  - `create_source_info(path, metadata) -> SourceInfo`
  - `create_synthetic_source_info(path, opts) -> SourceInfo`
- 迁移 `skills/loader.rs` 的 `SourceInfo` 为使用通用类型（type alias）
- 迁移 `resource/loader.rs` 的 `PromptTemplate.source_path` 为 `SourceInfo`
- 迁移 `commands.rs` 的 `SlashCommandInfo.source_path` 为 `SourceInfo`
- 所有迁移保持向后兼容（先加新字段，再逐步替换）

## Capabilities
- resource-discovery

## Impact
- 轻微破坏性：需要更新现有模块中对旧 SourceInfo 结构的引用。
- 无新依赖。
- 按 AGENTS.md「不做 BC shim」原则，一次性替换所有旧引用。
