# Tasks

- [x] 1. **specs**（spec）：`server-core` 的 `r1798` 明确 tape 帧的 `seq` 只表示事实位置快照、不唯一标识帧，客户端不得按 `seq` 去重或过滤 tape，须按到达顺序处理。测：`llman-sdd validate --all --strict`。
- [x] 2. **tests**（serve）：复核现有 tape 测试断言同批 tape 帧共享当前事实 `seq` 且不推进 journal；必要时补一条显式断言。测：`cargo test -p xylitol-serve tape`。
- [x] 3. **门禁**（repo）：`just qa` 全绿。测：`just qa`。
