# c735 tasks

- [x] 1. 扩展 `HookEvent`（provider 三缝 + agent/turn/message/session 等 pi 名）与 `event_matches`；修 `agent-hooks` main spec h1/h7 命名
- [x] 2. 实现 `ProviderHttpHooks`/`dispatch` 辅助：headers/body modify、after_response；Responses + Anthropic 接线；Completions 接受 hooks 句柄
- [x] 3. `BuildAgentOptions` + bootstrap 传入 `HooksConfig`；`build_provider` 闭包注入 `Arc<HookDispatcher>`；agent 持有同一 dispatcher
- [x] 4. ReAct：tool/context 桥接脚本；agent/turn/message 旁挂 dispatch（fail-open）
- [x] 5. Session/Driver：compact/fork/tree/switch/start/shutdown 能接则接
- [x] 6. 单测：empty noop、before_request modify、after_response 收到 status；BDD 填空 stub
- [x] 7. `llman sdd validate c735 --strict`；`cargo test` 相关 hooks + `cargo test --test bdd` hooks 场景
