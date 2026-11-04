# Tasks — TUI 流式渲染预算

- [x] 1. TextDelta/ThinkingDelta 不再逐事件 sync UiRoot；置 stream_sync_pending，Tick 合并同步。
- [x] 2. 保持 stable markdown prefix / streaming cache；每 tick 至多一次 paint。
- [x] 3. 不改变最终正文、thinking 与工具事件顺序；MessageEnd/AgentEnd 强制同步。
- [x] 4. Specs：app-tui-transcript 流式渲染预算。
- [x] 5. 测试：高频 delta 下 sync 次数下降；最终帧与逐事件一致；CPU 观察记录。
