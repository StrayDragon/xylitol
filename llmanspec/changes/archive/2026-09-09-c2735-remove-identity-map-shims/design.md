# Design: 删除 identity map

> Implemented。依赖 `c2700`。

## 1. 目标

bridge DTO 与 protocol 已同构的类型不再维护平行 enum + From。

## 2. 代码事实

- `src/protocol/model/meta.rs`：`TokenProvenance`、`ContextTokenEstimate`
- `src/infra/provider/map.rs`：identity `From`；另有 `to_xy_error` / `to_bridge_error`（保留）
- `src/protocol/message.rs`：`LlmMessage` = `AiBridgeMessage` 已是 alias 先例
- `infra/provider/mod.rs` re-export protocol 类型

`tool_spill` 符号当前仓库 **不存在**；不要为审计旧名复开模块。

## 3. 决策与做法

选择方案 A，与 `LlmMessage` 保持同一 alias 方向。bridge 已有同名 token DTO，因此补齐其 serde wire 形状与观测字符串方法；protocol 直接 `pub use` 这两个 DTO。主仓 provider map 删除 token identity `From`，token estimator 直接返回 bridge estimate。错误转换仍保留。

禁止 A+B 双类型。

## 4. 验证

provider map 单测改别名；`estimate_context_tokens` 路径绿。
