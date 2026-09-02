# Tasks：c2535 Staged Wire Import

- [x] t1 Host 暂存块：`dispatch_session_unary` writer 分支为 `import_jsonl` 增加 content→临时 input_path 暂存与双路清理；单测覆盖（content 导入成功 / input_path 直传不受影响 / 缺两者仍报 invalid_input）
- [x] t2 specs landing：`server-core.feature` 落 `@req:sr-imp1 @human` 规则 + `@executable` 场景；`steps_server.rs` 步骤 + `bindings_server.rs` 绑定转绿
- [x] t3 门禁：`just fmt` / `just lint` / `just test`（含既有 BDD 432 + 新 1）；`llman sdd validate --strict`
