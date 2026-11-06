# Tasks

## 1. cue 谓词与快照

- [ ] 1.1 `mcp_tools_pending` 以 settled 状态优先：`mcp_bootstrap_complete && tools_table_frozen` 时即使 `connecting_label` 残留也不得显示 cue。
- [ ] 1.2 `prepare_loaded_resources` 在 settled（complete 或 gate timeout）时清 `mcp_connecting_label`、置 `mcp_bootstrap_complete=true`，保留 `mcp_gate_notice` 与诊断。
- [ ] 1.3 header / status cue / `/mcp` 面板消费同一份 snapshot；connected/armed 后不得继续显示 connecting。

## 2. 刷新时机与 gate timeout

- [ ] 2.1 freeze / arm / gate timeout / reload / resume / 切会话后刷新 loaded-resources 并重算 cue；不得只在 turn 前刷新。
- [ ] 2.2 `join_session_prepare` 超时后由调用方收口为 frozen；timeout snapshot 为 complete/frozen，诊断保留。
- [ ] 2.3 gate timeout 后清 cue；用户 `/reload` 才重试，不自动重试。

## 3. connect timeout

- [ ] 3.1 `connect_timeout_ms` 覆盖 stdio spawn / initialize / handshake；超时收口 Failed / timed-out，不无限 connecting。
- [ ] 3.2 超时后 gate 不得永 Running；诊断进入 `/mcp`。

## 4. 测试与门禁

- [ ] 4.1 单测 / harness：pending → armed → cue 清除；gate timeout → cue 清除；hanging server → Failed；reload / resume 不粘旧 cue。
- [ ] 4.2 BDD：settled snapshot 清 cue；header 与面板一致。
- [ ] 4.3 tmux 真终端场景：首 turn 后无 pending cue；挂死 server 超时收口。
- [ ] 4.4 `just fmt`、相关 clippy、`just test`、`just qa`；BDD 新场景绑定并全绿。
