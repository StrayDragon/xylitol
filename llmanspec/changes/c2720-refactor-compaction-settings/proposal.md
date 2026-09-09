---
depends_on:
  - c2700-refactor-crate-public-surface
skip_specs_landing: true
---

# 折叠 compaction 设置三孪生

## Why

同一组键存在三处：`protocol::compaction_config::XyCompactionSettingsConfig`（serde 全 Option）、`agent::compaction::CompactionSettings`（运行时有默认）、`infra::config::types::CompactionSettingsSchema`（JSON Schema）。`From` 在 agent 侧。`SettingsManager` 与 `infra/config` 双轨用户偏好（c35 注释）。发布后三份类型会冻成假 SemVer 面。YAML 键保持；Rust 类型收成「文件 DTO 一份 + 运行时一份」或一份带 Default。

## What Changes

- 运行时只保留 `agent` 的 `CompactionSettings`（或改名但不双份语义）。
- 文件/schema：一份 serde DTO；schema 用 `schemars(with=…)` 或 infra 薄包装，禁止第三套字段表。
- 删除无调用的 SettingsManager 热路径若与 compaction 重复加载（以代码清单为准；不要误删仍被 bootstrap 使用的 merge）。
- YAML/JSON 键名 `enabled` / `reserveTokens` / `keepRecentTokens` **不变**（产品配置，不是兼容别名问题）。

## 非目标

- 不改 compaction 算法 / orchestrator。
- 不把 settings 热重载做成产品（见 c2715）。

## Capabilities

行为不变则 `skip_specs_landing: true`。若 live spec 钉了三类型名，绑定后改成「配置键 + 运行时 settings」。

## Impact

bootstrap `CompactionSettings::from(schema)` 路径简化。embed 若暴露配置类型，只留 protocol 或 agent 之一（与 `c2700` 公开面一致：外部走 embed 参数，不 reach infra schema）。

## 本批依赖

`c2700`。可与 c2705/c2715 并行。
