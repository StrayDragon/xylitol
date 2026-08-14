# Tasks: c2240-add-stream-node-timestamps

## 1. Branch binding 与 Specs landing

- [x] 1.1 `llman sdd change start c2240-add-stream-node-timestamps`
- [x] 1.2 [blocked-by: 1.1] 收紧 `app-tui-transcript` att8/att21/att24/att33：思考通道起止；resume 不用相邻条目墙钟
- [x] 1.3 [blocked-by: 1.1] `agent-runtime` 增加 streamTiming 节点落盘要求（单测，不扩 BDD step）
- [ ] 1.4 [blocked-by: 1.2] `llman sdd validate c2240-add-stream-node-timestamps --strict --no-check`

## 2. 在线钟与落盘

- [ ] 2.1 [blocked-by: 1.4] 把 ThoughtClock 扩成（或旁路）StreamNodeClock：写一次 + textEnded 覆盖
- [ ] 2.2 [blocked-by: 2.1] ReAct 在 AgentStart / TurnStart / chunk 切走点打戳；persist `streamTiming` + 派生 `thinkingElapsedSecs`；去掉顶层 thinking*AtMs 双写
- [ ] 2.3 [blocked-by: 2.2] TUI resume 读 elapsed 或 streamTiming 思考差；删除相邻条目墙钟回退
- [ ] 2.4 [blocked-by: 2.3] 单测：节点写一次、正文不膨胀 Thought、resume 用节点差、缺戳省略

## 3. 闸

- [ ] 3.1 [blocked-by: 2.4] `just fmt` + 相关 clippy；`cargo test --lib` 覆盖钟 / persist / TUI resume / activity fold Thought
