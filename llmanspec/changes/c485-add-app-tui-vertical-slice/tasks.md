# Tasks — c485-add-app-tui-vertical-slice

- [ ] 1.1 `ScriptedDriver`（测试用）：可脚本 `run` 事件流；记录 abort/steer/follow_up/clear_queue
- [ ] 1.2 薄编排 helper：消费 HostSession pending + Driver 流 → `HostEvent::Xy`（对齐 `run_host_loop` 顺序）
- [ ] 1.3 合成 harness：H1–H9（H10 复用既有 preflight 测或补缺）
- [ ] 2.1 `tests/tui_e2e`：产品二进制 PTY smoke（Fake + `--trust` → Hello → `/exit`）；`#[ignore]`
- [ ] 2.2 `just test-tui-e2e-pty`（或注释）可跑通新产品用例
- [x] 3.1 更新 `src/app/tui/{AGENTS,DESIGN}.md`：下一刀 / Overview 指向 c485
- [ ] 3.2 `llman sdd validate c485-add-app-tui-vertical-slice --strict`
