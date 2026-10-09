# Tasks

## T1 — 行为改造
- [x] `SessionSlot::sync_model_downlink`：`append_and_push` → transient 推送（与 `push_resources` 同族，不消耗 seq）。

## T2 — BDD 场景 + 回归
- [x] 场景：二次 attach journal max_seq 不变且订阅者收到 ModelSelect；步骤对落地 `tests/bdd/`。
- [x] r1926 verified-by 由 `src/app/server/host.rs` 改锚步骤/回归 fn。

## T3 — 门禁收口
- [x] fmt / clippy / `just qa` 绿 + `llman-sdd validate --strict` 0。
