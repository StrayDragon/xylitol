# Tasks

- [x] 1. **specs**（agent-runtime）：扩展 `r2962`（multiple-before-tool-gates），写明 `deny_reason` 首次设置即锁定、后置不得清空/改写、链终态以首个非空原因拒绝。测：`llman-sdd validate c2990-clarify-deny-reason-chain-semantics`。
- [x] 2. **runtime**（middleware）：`run_stage` 增加 settle 回调；`run_before_tool` 锁定首个非空 `deny_reason`，其他阶段传空操作。测：`cargo test -p xylitol-runtime middleware`。
- [x] 3. **tests**（runtime）：补链测试——先 deny 后 clear 仍拒绝；后置可读取原因；先 deny 后改写仍保留首个原因。测：`cargo test -p xylitol-runtime`。
- [x] 4. **门禁**（repo）：`llman-sdd validate --strict`、`just qa` 全绿。测：`just qa`。
