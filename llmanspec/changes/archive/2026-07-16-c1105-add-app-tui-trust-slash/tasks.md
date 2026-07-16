# Tasks — c1105-add-app-tui-trust-slash

## 1. Seam + slash

- [x] 1.1 `Driver::persist_project_trust` + `InProcessDriver` / ScriptedDriver
- [x] 1.2 `commands` / product catalog：`/trust`[+ parent|deny]；busy 拒绝
- [x] 1.3 `effects/slash`：写盘 + 提示 reload/restart；MUST NOT `reload_runtime`

## 2. 测试

- [x] 2.1 harness：idle 成功路径可观测；busy 不写；不触发 reload
- [x] 2.2（可选）InProcess 单元：temp trust dir 写盘

## 3. 校验

- [x] 3.1 `LLMANSPEC_BASE_REF=main llman sdd validate c1105-… --no-interactive`
- [x] 3.2 `just lint` + 相关测（或 `just qa`）
