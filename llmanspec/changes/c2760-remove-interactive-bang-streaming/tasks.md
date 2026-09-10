# Tasks: c2760-remove-interactive-bang-streaming

> pre-start。先 Branch binding（`change start`），再在绑定分支改 live specs（Specs landing），最后 apply。

## 1. 闸与合约

- [ ] 1.1 `change start` 后列出将改的 live capabilities：`app-tui-bridge` / `app-tui-transcript` / `app-tui-host`。
- [ ] 1.2 [blocked-by: 1.1] Specs landing：按 design §4 改写三处条款；proposal frontmatter 加 `rules_edit_acked: true`。
- [ ] 1.3 [blocked-by: 1.2] `llman sdd validate c2760-remove-interactive-bang-streaming --strict --no-check`。

## 2. 去流式通道

- [ ] 2.1 [blocked-by: 1.2] `XyDriver` 删 `execute_bash` 声明；in-process trait 实现改为 inherent `execute_bash_impl(command, exclude_from_context)`（body 同源，传 `None`）；executor 的 `Command::Bash` 分支改调它。
- [ ] 2.2 [blocked-by: 2.1] 删 remote / harness 的 trait `execute_bash` 实现（remote 的 Bash 走 executor 已有）。
- [ ] 2.3 [blocked-by: 2.2] TUI `run_interactive_bang`：`select!` 去 `chunk_rx` 臂，改 `dispatch(Command::Bash)`；保留 Esc abort / busy 拒绝 / 终态渲染。
- [ ] 2.4 [blocked-by: 2.3] 删 bridge `append_bash_output` + `bash_utf8_pending`；删 host `append_bash_chunk` 与 chunk dirty 分支。

## 3. 测试

- [ ] 3.1 [blocked-by: 2.4] harness：删/改 c669 流式用例；保留 bash_calls、第二 bang 硬拒绝、Esc cancelled 断言。
- [ ] 3.2 [blocked-by: 3.1] 确认 agent bash tool 链路测试不动（`steps_agent_tools::test_bash_cancel` 等）。
- [ ] 3.3 [blocked-by: 3.2] `just test-tui` + `just qa`。

## 4. 收尾

- [ ] 4.1 [blocked-by: 3.3] 交接：注明 `XyBashExecutor` 的 `chunk_tx` 端口保留（agent 链路），XyDriver 已无会话操作方法。
