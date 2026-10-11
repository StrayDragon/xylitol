---
title: 同 Host 第二路 TUI attach：不跟看打字机（未决）
depends_on:
  - c2893-fix-tui-attach-working-run-agentend
---

# 同 Host 第二路 TUI attach：不跟看打字机（未决）

## Why

c2893（session 写占用改内核锁）之后：同一 Host 上两个 TUI `--session` 同一会话，A 正在对话时 B 再问，会同时看到 **忙/插话提示**（符合预期）和 **与 A 相同的推流打字机**（不符合预期）。

原因不是内核锁漏了。占用者是 Host 里该 session 的写引擎；同进程两条连接共享一个 `SessionSlot`，下行 `session/event` 扇出给全部 mux 订阅者。r1514（一 session 一写者）写过「再 attach MUST 只读恢复」，又用「尚未提供多客户端 attach 时，单进程默认路径视为已满足」放过。用户倾向是：B **不该跟看直播**，而该 **重新 resume**——**尚未拍板**。

## What Changes

草案，不实施。propose 时才锁行为。候选方向（互斥，见下节）：

- 第二路拒绝直播，提示会话正被另一端使用，结束后再 resume，从 journal 冷恢复。
- 或第二路只读快照（`get_messages`），无 live `TextDelta`，写入失败。
- 或保持扇出（双窗跟看）——与当前实机一致，需显式接受。

## 非目标（草案）

- 不改 c2893 跨进程 `session.lock` / linger / 砍令牌。
- 不在本票里做 gpui 第二面、跨 Host 抢锁、tmux 式结对编程（除非重新讨论选定跟看）。
- 不实施（仅 `proposal.md`）。

## 重新讨论要点（propose 前 MUST 再开）

> **未决定。处理本 change 时先逐条过完本节，再写 specs / design。不要把「用户倾向」当成已定 MUST。**

1. **产品形态三选一（核心）**
   - **(A) 强制再 resume**：第二路不得订 live 带；提示「另一端正在用，结束后再 resume」。A 断开后 B 显式 `--session` 才看到完整记录。
   - **(B) 只读观察者**：第二路可冷恢复历史，无打字机；写入失败并说明已有写者。A 结束后 B 是否自动升格写者，还是仍须再 resume？
   - **(C) 直播扇出（现状）**：双 TUI = 同一事件总线，跟看是特性。提示仍可保留。需显式接受，否则不要当默认。
   - 实机反馈倾向 (A)，**未拍板**。

2. **什么叫「已被占用」？**
   - 只要有一条 mux 订阅（B 一 attach 就挡），还是仅 `run_inflight`（A 空闲挂着时 B 可以变成写者）？
   - 与 c2893 linger：A Ctrl+C 后宽限窗口内，B resume 算续占还是冲突？

3. **B 在 A 回合中再问，提示之后做什么？**
   - 大声失败（新根提交拒绝）——对齐「忙碌时二次根提交有明确拒绝」（插话续跑文）。
   - 还是变成 A 回合的 **插话 / 续跑**（今日 TUI Busy→Steer 很可能走这条，B 会误以为自己开了一轮）？
   - 两路 TUI 是否共享同一条插话队列、同一份 `run_inflight`？

4. **B 被挡住时屏幕上有什么？**
   - 空白 + 提示，什么 transcript 都没有？
   - 还是 `get_messages` 冷快照（无后续 delta），提示「不要当写者」？
   - 冷快照若含 A **尚未 AgentEnd** 的半截助手行，算不算另一种打字机？

5. **r1514 那句豁免怎么收？**
   - 「尚未提供多客户端 attach 时，单进程默认路径 MUST 视为已满足」——本票是否删掉，改成真正的第二 attach 行为？
   - 还是把「同 Host 双 TUI」继续排除在「多客户端」之外，只修 TUI 端不画别人的 delta？
   - 合约落在 `layer-architecture` r1514、`server-core` r1793 旁路，还是新 requirement？

6. **端形态边界**
   - 两个终端进程 attach 同一 Host，与 **同一 TUI 两个 tab**（c2880 tui-tabs）resume 同一 session：同一套规则还是 tab 允许跟看、跨进程 TTY 不允许？
   - in-process Print / 嵌入与 HTTP 第二客户端是否同一语义？

7. **实现气味（有结论再碰代码）**
   - `replay_or_resync` 把 journal **广播给全部**订阅者：B 一订，A 也可能吃到重放突发。
   - 单 writer 订阅 vs 客户端 `skip_cold_replay` 吞掉别人的 delta：Host 裁扇出更硬，端侧过滤易漏（B 若 `run()` 会关掉 skip）。
   - 第二 `prompt` 在 `run_inflight` 时仍可能再 `d.run()`：是排队、拒绝，还是今日的重叠流？

## Capabilities（propose 时再钉）

- `layer-architecture`（r1514 一 session 一写者 / 第二 attach）
- `server-core`（订阅扇出、占用与 `run_inflight`）
- `app-tui-bridge` / `app-tui-host`（Busy 提示 vs 打字机；是否强制再 resume）

## Impact

- 拍板前：同 Host 双 TUI 仍共享打字机（c2893 后现状）。
- 若选 (A)：第二路不再误以为自己在写同一轮；代价是不能双窗盯同一回合。
- 若选 (B)：可看历史、不能写、无直播。
- 若选 (C)：文档改口为「同 Host 多端跟看是特性」，r1514 豁免保留或改写。

## Further Notes

- 触发：c2893 落地后实机，两 TUI resume 同一 session。
- 占用设计：`llmanspec/changes/c2893-fix-tui-attach-working-run-agentend/design.md`（占用者 = Host 写引擎，不是 TUI 进程）。
- 扇出：Host 槽位对全部 mux 订阅者 `broadcast`；`run_inflight` 管同进程串行，不管谁看见 delta。
- 只读恢复旧句：r1514；跨进程锁：r1793。本票是 **同 Host 连接级**，不是再拧 `session.lock`。
- 插话 vs 根提交：`docs/architecture/插话续跑与中止.md`。
- 多 client 矩阵：`docs/architecture/库与多客户端.md`（未写双 TUI 跟看）。
