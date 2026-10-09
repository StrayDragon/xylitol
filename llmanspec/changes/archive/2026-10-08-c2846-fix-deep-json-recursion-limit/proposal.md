---
depends_on: [ c2845-fix-deep-session-tree-recursion ]
needs_specs_change: true
branch: pr/2026-10-bdd-infra-and-contracts
base_branch: main
base_sha: dc379efb8430dae278ffc66c7bdd7e56b3faa202
---

# 修复深树 JSON 的 serde 默认递归上限泄漏（两轨）

## Why

c2845 使 `session_tree` 走 RawOk 后，深树从「binary codec 递归」转为「客户端 serde_json 解析」（serde 小帧，深度安全的前提成立）。但 serde_json **默认解析递归上限 128 层**：实机深会话 188 层 → `from_str` 失败并被 `.unwrap_or(Value::Null)` **静默降级 Null** → 下游报 `invalid type: null, expected a sequence`。

## What Changes

1. **serde_json 启用 `unbounded_depth`**（Cargo.toml，默认仍 128 受限，仅显式调用豁免）。
2. **v3 轨**（`host_client/wire_v3_client.rs`）：`payload_value` 的 RawOk 臂改用 `parse_raw_json`（禁递归上限解析），保留 r1907 Null 降级语义。
3. **JSON 轨**（`protocol/wire/codec.rs`）：`decode / decode_str` 改走 `parse_json_value`（同法禁上限）——两轨车同轨，r1908 对拍适用。
4. **spec `server-core` r1922** + 场景 `v3-deep-session-tree-raw-parity`：深链（>128）双轨取树 MUST 等价且完整还原（非 Null）——CI 级回归守卫（本次实机踩坑的自动化兜底）。

## Capabilities

- `wire v3` / `wire jsonrpc`（线载荷解析）、`test-infra`（wire 对拍）

## Impact / 风险

- 只放宽「深度」维度；载荷仍受服务端信任与真实树深约束（帧小）。失败仍按既有语义降级。
- 与 c2845 同属「深度无关」承诺的收口。
