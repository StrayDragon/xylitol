# Design: compaction 设置折叠

> Designed / pre-start。依赖 `c2700`。

## 1. 目标

compaction 配置：**磁盘形状一份、运行时一份**。禁止 protocol + agent + infra schema 三份字段清单。

## 2. 代码事实

| 类型 | 位置 | 角色 |
|---|---|---|
| `XyCompactionSettingsConfig` | `src/protocol/compaction_config.rs` | 全 Option serde，camelCase |
| `CompactionSettings` | `src/agent/compaction/settings.rs` | enabled/reserve/keep_recent；`From` config |
| `CompactionSettingsSchema` | `src/infra/config/types.rs` ~L901 | JSON Schema 孪生 |
| bootstrap | `src/app/core/bootstrap.rs` Step 3b2 | files → schema → `From` → Runtime |

`protocol` 注释已写 schema 应 derived on infra DTO（c510），与「三孪生」现状打架。本 change 落地 c510 意图。

## 3. 推荐形状

- **磁盘**：保留 protocol `XyCompactionSettingsConfig` 为 SSOT serde（ports/protocol 可被 infra 用）。
- **schema**：infra 对同一类型 `schemars`，删除手写 `CompactionSettingsSchema` 字段复制。
- **运行时**：agent `CompactionSettings` 保持非 Option + 默认；`From` 留一处。

不要把运行时类型塞进 protocol（避免 agent 默认值泄漏到 wire）。

## 4. SettingsManager

先 `rg 'SettingsManager'`：若 compaction 只走 `infra/config` + bootstrap，不要为「统一」去发明 SettingsManager 接线。闲置 API 能删则删（死码分诊：真死删）。

## 5. 验证

config validate 单测（`src/infra/config/validate.rs`）仍绿；键名与 `configs/example.yaml` 生成模板一致。不改 example 语义。
