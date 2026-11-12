# Design — 压缩边界与一次性通知（2026-11-13 复核后定稿）

## 1. 目标与边界

- 边界推导单点；一次性通知 emit 之后才置位；切点哨兵显式 Option；压缩后不变量重建单次加载。
- 边界：不改切点算法本身；不动 compaction 配置；不动 runtime 侧 overflow 复位（随 c3240）。

## 2. 决策摘要（2026-11-13 复核后全部定稿）

| 决策 | 内容 | 状态 |
|---|---|---|
| 边界单点 | boundary_start(entries) -> usize 供 prepare/compact 共用；「已压缩」判定改为跳过非消息 entry 找最后一条 context entry，若为 Compaction 则 Already compacted | 定稿 |
| 通知门 | take_fallback_notice 只组装文案、不置位；两处调用方在 event_sink.emit(...).await 返回后置位。emit 无 Result（fire-and-forget），故无重试循环，失败可观测性受 sink API 限制 | 定稿（修正：emit 不可观测成功/失败，retry 无意义；改为「emit 后置位」） |
| overflow 复位 | runtime 侧复位点随 c3240，本 change 不动 runtime | 定稿 |
| 切点哨兵 | CutPointResult { first_kept_entry_index: usize, split_turn: Option<usize> }；isize(-1) 哨兵删除；两处解释改 if let Some(turn_start) | 定稿 |
| 不变量重建 | ensure_state_after_compact 单次加载、逐项补齐（appended 后并入内存 entries 并重算 cut）、失败 log + 返回 bool；无持久 retry marker（无消费者，下次压缩重试） | 定稿（修正：待重试标记无消费方） |
| rider | 角色白名单两份语义不同（turn-start=user vs 可摘要角色 user/toolResult/bashExecution/assistant），不合并；0 非未知哨兵（非消息 entry 计 0 是「不计入上下文」语义） | 定稿（复核修正） |

## 3. 通知时序

组装 → emit CompactionEnd（含 notice）→ 置 fallback_notice_emitted / floor_notice_emitted。emit 前崩溃或 sink 丢弃 → 标志保持未置位，下轮压缩可再发。

## 4. 测试边界（seam）

- 单测：两调用方边界一致性（同 entries 同 boundary_start）；末条为非消息 entry 时的已压缩判定；切点 split_turn Some/None 解释一致；ensure 单次加载后四类状态齐全。
- BDD：domain-compaction 场景回归。

## 5. 风险与回滚

- 通知时序改为 emit 后置位；回滚三片独立。

## 7. 实施前复核（2026-11-13）

| 结论 | 复核 | 证据 |
|---|---|---|
| boundary_start 双份实现且末条判定不同 | 成立 | lib.rs:104-118（prepare）与 :177-193（compact）；:100 只看 entries.last() |
| 一次性通知先置位后发 | 成立（emit 无 Result，失败不可观测） | orchestrator.rs:446-464 置位；:102-118、:419-433 emit；sdk ports/event.rs:9-11 emit -> () |
| 切点 isize(-1) + is_split_turn 双字段 | 成立 | cut_detector.rs:6-10；lib.rs:130-134、224-228 |
| 压缩后四条 ensure 各自重载 | 成立 | lib.rs:353-356；368-537 每函数各自 load_leaf_branch |
| 角色白名单两份 | 成立但语义不同 | cut_detector.rs:181-182、204；llm_summarizer.rs:142、149 |
| 0 兼作未知哨兵 | 不成立为缺陷 | cut_detector.rs:14-33 非消息 entry 计 0 = 不计入上下文 |
