# Tasks

测试边界：见 `design.md`（Host unary 处理器 + 产品 TUI 合成 harness，均复用既有边界）。行为代码已基本落地，本票以合约落地 + 护栏场景为主；场景跑红即修码。

## 1. 合约落地（Specs landing）

- [x] 1.1 `change start` 绑定 `sdd/c2307-fix-tui-attach-cold-restore`（规划壳已提交默认分支，树干净）
- [x] 1.2 live specs：`server-core` 新增 `w8` 冷恢复投影（快照 unary 是 transcript 投影源；journal 重放 MUST NOT 作冷恢复来源；sr4/w5/w6 不动；`w7` 已被 WS 事件推送占用故顺延）；`app-tui-host` 新增 `ath36` attach 恢复渲染（一次重建、完整历史、Idle、无假 spinner；冷订窗磁带不渲染）
- [x] 1.3 commit Specs landing；结构过闸（`readyToImplement=true`）

## 2. server-core：w8 场景（Host unary seam）

- [x] 2.1 [blocked-by: 1.3] 失败测/场景：`server-ws.feature` 挂 `@req:w8`——会话有历史条目时，冷订（last_seq=0）后 `get_messages` 返回全量条目一次投影
- [x] 2.2 场景：journal 重放窗内的实况磁带事件不改变 `get_messages` 快照结果（快照与重放互不污染）
- [x] 2.3 若场景暴露真实缺口（如投影缺失/重复），在本任务内修码至绿；`cargo test --test bdd` 过

## 3. app-tui-host：ath36 护栏（按 ath28-unit 先例：harness/驱动级测试，非 BDD）

- [x] 3.1 驱动级测试 `cold_subscribe_drops_replay_tape_and_stays_live`：冷订重放窗内 TextDelta 磁带不下行；ack 后 live 流继续（`.feature` 不挂 ath36——app-tui-host.feature 现状即全无 BDD 绑定，导出 crate 内 harness 属扩面，遵循 ath28-unit 先例）
- [x] 3.2 harness 测试 `harness_resume_snapshot_rebuild_renders_full_history_once_idle`：Resume 快照一次重建完整历史、Idle、无假 spinner
- [x] 3.3 lib + bdd 全量绿

## 4. 对拍收尾

- [x] 4.1 真实 attach 手测迁移清单 §4 步骤 6：`--session 05eb4dbd…` 冷恢复 T+2s 一帧完整历史（两轮问答 + restored 滚动提示）、零 spinner 字符、footer token 已恢复；T+6s 稳定无变化
- [x] 4.2 勾根 `_TUI_MIGRATED_TODO.md` P0「A1 快照」项；「A7 后续」（模型侧工具多工作区）移入 P1——不挡本票
