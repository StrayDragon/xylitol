# 设计：深树 JSON 解析深度无关（两轨车同轨）

## 根因链（实机确认）

1. c2845 把 session_tree 移到 RawOk；服务端序列化无递归上限（/rpc 文本返回完整 188 层树）。
2. 客户端 v3 RawOk 臂 `serde_json::from_str(&raw.json).unwrap_or(Value::Null)` —— serde_json 默认解析递归上限 **128**，188 层 → `recursion limit exceeded` → 静默 Null → `data.get("tree")` → `from_value(null)` → "invalid type: null, expected a sequence"。
3. 同一深树经 JSON 轨客户端 `codec::decode_str`（`from_str`）同样撞 128 —— BDD 深链场景实测抓出（`json session_tree: recursion limit exceeded`）。

## 方案

- Cargo.toml serde_json 启用 `unbounded_depth`（默认行为不变，仅显式 `disable_recursion_limit` 豁免）。
- v3 轨 `payload_value` RawOk 臂 → `parse_raw_json`（禁限解析；失败仍按 r1907 降级 Null）。
- JSON 轨 `codec::decode/decode_str` → `parse_json_value`（同法）。
- 回归守卫：单测（>128 深度 RawOk 解析不降级）+ BDD `v3-deep-session-tree-raw-parity`（160 层深链双轨等值、非 Null、深度>128）。测试夹具自身解析同样禁限。

## 明确不做

- 不改服务端序列化（无上限，/rpc 已验证）。
- 不全局禁用所有 serde_json 解析（仅 wire 客户端两处载荷解析）。
