# Tasks

## T1 — 解析修复（两轨）
- [done] Cargo.toml serde_json `unbounded_depth`。
- [done] `wire_v3_client.rs parse_raw_json`（禁限）+ RawOk 臂接入。
- [done] `codec.rs parse_json_value`（禁限）+ decode/decode_str 接入。
- [done] 单测 `raw_ok_parses_beyond_default_recursion_limit`（150 层）。
- 校验：lib 1620 绿。

## T2 — spec + BDD
- [done] server-core r1922 + 场景 `v3-deep-session-tree-raw-parity`（深链 given + then 深度断言）+ 绑定。
- 校验：单场景绿（160 层双轨等值非 Null）。

## T3 — 门禁收口
- [done] fmt / clippy 0 / BDD 957/957 / validate 0。
- 收口：finalize c2846（不带 --no-check）+ push。
