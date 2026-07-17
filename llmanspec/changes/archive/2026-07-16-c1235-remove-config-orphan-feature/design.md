# Design — c1235-remove-config-orphan-feature

## 决策

删除而非迁移，与 c1230（event-bus）一致。config.feature 的 7 个场景测的是配置加载
管线（YAML 三层合并、ConfigValue `$VAR`/`!cmd` 解析、模型/provider 字段、默认值），这些
已被 src/infra/config 与 src/infra/settings 的 60+ 单测覆盖（value.rs/env+cmd 解析、
types.rs/模型解析、loader.rs/加载、manager.rs/deep_merge）。

迁移到 solidify 链路需新建 runtime-config scenario + 重写 step，与单测重复，违反测试分层。

## 验证证据

- `rg 'config\.feature' tests/ src/` → 零引用
- 删除后 `cargo test --test bdd` → 100 passed, 0 回归
- `cargo test --lib infra::config` → value.rs/types.rs/loader.rs 单测全过
