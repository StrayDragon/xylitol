# Tasks

## 1. 配置

- [x] 1.1 `SubagentConfig` + `AppConfig.subagent`（Option，缺省零装配）。
- [x] 1.2 `just gen-config-schema` / `just gen-config-example`。
- [x] 1.3 README 说明。

## 2. 工具与隔离 runtime

- [x] 2.1 `SubagentTool`（`subagent_readonly`）：`prompt` 参数与 schema。
- [x] 2.2 `SubagentFactory`：`BootOptions` + allowed tools + 可注入 env/cwd。
- [x] 2.3 `bootstrap_with` 另建临时 runtime；`static_tools` 为空。
- [x] 2.4 只读工具过滤 + `set_tools`。
- [x] 2.5 消费事件取最后 assistant 文本；空文本报错。
- [x] 2.6 父级 `ctx.cancel` → `driver.abort()` + `XyToolError::Aborted`。

## 3. 装配

- [x] 3.1 `boot_ctx` 在 mount core 后注册工具。
- [x] 3.2 `static_tools_from_ctx` 带入父工具集；子代理不递归。

## 4. 规格与 BDD

- [x] 4.1 `agent-subagent` 两条规则与场景。
- [x] 4.2 BDD steps + bindings：零装配与注册。
- [x] 4.3 kernel 单测：只读过滤、缺 prompt、fake provider 返回文本。

## 5. 门禁与收尾

- [x] 5.1 `just qa` 全绿。
- [x] 5.2 `just qa-e2e` 全绿。
- [x] 5.3 `llman-sdd validate c3005-add-subagent-readonly --check --strict` 通过。
- [x] 5.4 verify 无 CRITICAL，finalize 归档；PR 用 rebase merge。
