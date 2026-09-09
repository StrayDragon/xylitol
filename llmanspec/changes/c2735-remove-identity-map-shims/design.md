# Design: 删除 identity map

> Designed / pre-start。依赖 `c2700`。

## 1. 目标

bridge DTO 与 protocol 已同构的类型不再维护平行 enum + From。

## 2. 代码事实

- `src/protocol/model/meta.rs`：`TokenProvenance`、`ContextTokenEstimate`
- `src/infra/provider/map.rs`：identity `From`；另有 `to_xy_error` / `to_bridge_error`（保留）
- `src/protocol/message.rs`：`LlmMessage` = `AiBridgeMessage` 已是 alias 先例
- `infra/provider/mod.rs` re-export protocol 类型

`tool_spill` 符号当前仓库 **不存在**；不要为审计旧名复开模块。

## 3. 做法

方案 A（与 LlmMessage 一致）：protocol `pub use xylitol_ai_bridge::dto::{TokenProvenance, ContextTokenEstimate}`（若 bridge 已有同名）。注意 protocol 依赖 ai-bridge 是否已存在（message.rs 已依赖）。

方案 B：只留 protocol 类型，adapter 返回时直接构造，删除 From 文件中的 match。

选 A 当 bridge 已是 SSOT；选 B 当不想 protocol 再多 export。禁止 A+B 双类型。

## 4. 验证

provider map 单测改别名；`estimate_context_tokens` 路径绿。
