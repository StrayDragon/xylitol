---
change_id: c1235-remove-config-orphan-feature
title: "移除 config 孤儿 feature（配置加载由单测覆盖）"
status: proposal
priority: 1235
apply_band: P3-core
depends_on:
  - c1230-remove-event-bus-orphan-feature
author: agent
track: R
wave: bdd-migration
---

# c1235-remove-config-orphan-feature

## Why

`tests/features/config.feature` 含 7 个场景（三层合并、模型解析、base_url、环境变量插值、
外部命令密钥、设置验证、默认值），从未在 `tests/bdd.rs` 或 `src/` 实现 step 或绑定——纯孤儿。
这些配置加载行为已由 `src/infra/config/`（value.rs 29 单测、types.rs 9、loader.rs 6）与
`src/infra/settings/`（manager.rs 11，含 deep_merge）的单元测试充分覆盖。

与 c1230（event-bus）同理：按测试分层原则，配置加载是内部管线，单测已保障，孤儿 BDD 是债务。

## What Changes

1. 删除 `tests/features/config.feature`（7 个未实现场景）。
2. delta `test-bdd` add tb4：明确"配置加载 MUST 由单测覆盖，MUST NOT 保留孤儿 feature"。

## Capabilities

- `test-bdd` — add tb4（配置 BDD 分层合约）

## Impact

- feature 文件 20 → 19。
- bdd 全量：100 passed，0 回归。
