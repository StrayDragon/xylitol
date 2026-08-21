---
depends_on: []
---

# Design：attach 冷恢复快照投影

## 现状事实（2026-08-21 代码）

1. **冷订丢弃已落地**：Remote 冷订（`last_seq=0`）置 `skip_cold_replay`，恢复窗内丢弃 Agent 实况磁带类事件，收到 `session/subscribed` ack（重放结束、进入 live）后清除。
2. **快照投影已存在**：CLI `--session`（无本地 store 分支）与 TUI 内 Resume 切换都走 `get_messages` unary → `apply_*_session` 一次重建 transcript。
3. **Host 行为未变**：对 `last_seq=0` 仍逐条重放 journal（sr4 字面语义），带宽浪费但用户不可见。
4. **时序自洽**：订阅 → 快照重建（全量替换，T1 快照 ⊇ [订阅, T1] 间已提交事件）→ live 增量追加。无丢增量窗口。
5. **合约缺口**：以上语义零 specs。specs 只有 sr4 续传、w5/w6 resync；`get_messages` 作为 transcript 投影源无处声明。

## 决策

### D1：冷订磁带丢弃留在客户端，不搬 Host

- 客户端丢弃已落地且有单测；搬 Host 需要新的订阅语义参数（协议面扩大），收益只有带宽。
- sr4 面保持字面不变：`subscribe(last_seq)` 重放语义不动；恢复语义由「投影走快照」承担。
- 大会话重放带宽优化候补（未来可加订阅参数），不在本票。

### D2：快照形状用现有 `get_messages`，不加 seq 游标

- 应用顺序（订阅→重建→增量追加）配合「重建=全量替换」已保证一致性（事实 4）。
- 「快照+游标原子衔接」是极端竞态/效率优化，出现真实丢增量回归再立项。

### D3：不新增 loading 中间态

首帧先画（welcome chrome 不被 JSONL 加载阻塞）；恢复投影完成后一次重建。禁止用 spinner 掩盖回放。

## Spec 增量

| capability | 增量 |
|---|---|
| `server-core` | 新 req：会话 transcript 投影源 = 消息快照 unary（全量条目）；journal 实况重放（sr4）MUST NOT 作为冷恢复的 transcript 投影来源 |
| `app-tui-host` | 新 req：attach 恢复（`--session` 启动与空闲 Resume 切换）MUST 以快照一次重建 transcript（完整历史、Idle、无假 spinner）；恢复窗内 MUST NOT 把冷订实况磁带当 Agent 事件渲染 |

既有 sr4 / w5 / w6 / ath35 不改。

## 测试边界（seam）

| 边界 | 场景 |
|---|---|
| Host unary 处理器（`handle_unary`，进程内 HostState） | 冷订后 `get_messages` 返回全量条目；快照与 journal 重放互不污染 |
| 产品 TUI 合成 harness（`ScriptedDriver` + HostSession，既有 BDD 边界） | 恢复窗内磁带事件不渲染；resume 切换一次重建、Idle、无假 spinner |

两处都复用既有 harness 边界，不发明新缝。

## 风险与非目标

见 proposal「非目标」。最大风险是把本票做成协议改造——刻意收窄：不改 wire 方法表、不改 sr4/w5/w6、不加参数。
