# design — c510 domain / config schema 边界

## 决策

- **domain**：只保留业务 + serde；去掉 `schemars::JsonSchema`。
- **infra/config & settings**：需要 JSON Schema 的 DTO 在此 derive。

## 映射

若 domain 与 config 字段同构：

- 优先让「仅配置用」结构只活在 infra；
- 若 domain 仍需同构类型：infra DTO `From`/`Into` domain，schema 只挂 DTO。

## 已知位点（实施时复核）

- `src/domain/model.rs`（`XyModelKind` 等）
- `src/domain/compaction_config.rs`（`XyCompactionSettingsConfig`）

以 `rg JsonSchema src/domain` 清零为验收。
