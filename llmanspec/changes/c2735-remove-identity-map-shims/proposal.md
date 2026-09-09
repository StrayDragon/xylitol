---
depends_on:
  - c2700-refactor-crate-public-surface
skip_specs_landing: true
---

# 删除 identity map：token 类型双 From 与同类 shim

## Why

`infra/provider/map.rs` 对 `TokenProvenance` / `ContextTokenEstimate` 做 bridge → protocol 的逐变体 `From`。两边变体已 1:1（identity）。`protocol::message` 已 `pub use` bridge `LlmMessage`。再养一份 From 是发布后的假稳定转换层。同类「只为 crate 边界再包一层」的 shim 一并删（审计时的 tool_spill 若已不存在则跳过）。

## What Changes

- token 估计：优先 `pub use` bridge 类型为 protocol 别名（与 `LlmMessage` 同模式），或单一 newtype 无 match From。
- 删除 `impl From<AiBridgeTokenProvenance> for TokenProvenance` 这类 identity match。
- 扫 `infra/provider/map.rs` 其它 1:1 map；错误转换（`XyError` ↔ `AiBridgeError`）**保留**（非 identity）。
- `src/AGENTS.md` 消息投影：LLM 叶走 alias，禁止再引入平行 enum。

## 非目标

- 不改 tokenizer 行为/ provenance 语义。
- 不把 ai-bridge 依赖抬进 app。

## Capabilities

内部类型别名。`skip_specs_landing: true`。

## Impact

调用点改 import。无 wire 变化。

## 本批依赖

`c2700`（infra 已 crate-private，改 map 不碰外部 API）。可与 c2715/c2720 并行。
