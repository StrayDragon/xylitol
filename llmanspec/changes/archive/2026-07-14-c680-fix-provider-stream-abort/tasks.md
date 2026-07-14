# Tasks — c680-fix-provider-stream-abort

- [x] 1. Delta specs 校验：`LLMANSPEC_BASE_REF=origin/main llman sdd validate c680-fix-provider-stream-abort --strict --no-interactive`
- [x] 2. PoC：`tests/provider_http_stream_abort.rs`（reqwest drop + Anthropic adapter drop 断连）
- [x] 3. ReAct：`select!` 竞态 cancel vs `call_with_retry` / chunk loop；abort 时 drop stream
- [x] 4. 单测：`abort_mid_stream_stops_polling_model_chunks`
- [x] 5. BDD：Driver 首 TextDelta 后 abort → aborted 且 TextDelta 远少于脚本量
- [x] 6. `just fmt` + 相关测试绿；可选真机：用户 Esc 后看服务端是否停生成
