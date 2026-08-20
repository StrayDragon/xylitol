# Tasks

测试边界：见 `design.md`（Host unary 处理器 + 产品 TUI 合成 harness，均复用既有边界）。行为代码已基本落地，本票以合约落地 + 护栏场景为主；场景跑红即修码。

## 1. 合约落地（Specs landing）

- [ ] 1.1 `change start` 绑定 `sdd/c2307-fix-tui-attach-cold-restore`（规划壳已提交默认分支，树干净）
- [ ] 1.2 live specs：`server-core` 新增 `w7` 冷恢复投影（快照 unary 是 transcript 投影源；journal 重放 MUST NOT 作冷恢复来源；sr4/w5/w6 不动）；`app-tui-host` 新增 `ath36` attach 恢复渲染（一次重建、完整历史、Idle、无假 spinner；冷订窗磁带不渲染）
- [ ] 1.3 commit Specs landing；`llman sdd validate c2307-fix-tui-attach-cold-restore --strict --no-check --no-interactive` 结构过闸

## 2. server-core：w7 场景（Host unary seam）

- [ ] 2.1 [blocked-by: 1.3] 失败测/场景：`server-ws.feature` 挂 `@req:w7`——会话有历史条目时，冷订（last_seq=0）后 `get_messages` 返回全量条目一次投影
- [ ] 2.2 场景：journal 重放窗内的实况磁带事件不改变 `get_messages` 快照结果（快照与重放互不污染）
- [ ] 2.3 若场景暴露真实缺口（如投影缺失/重复），在本任务内修码至绿；`cargo test --test bdd` 过

## 3. app-tui-host：ath36 场景（合成 harness）

- [ ] 3.1 [blocked-by: 1.3] 失败测/场景：`app-tui-host.feature` 挂 `@req:ath36`——attach 恢复窗内注入 Agent 实况磁带（TextDelta/AgentStart）MUST NOT 渲染进 transcript、不出假 spinner
- [ ] 3.2 场景：Resume 切换经快照一次重建——完整历史一帧可见、Idle、无逐条回放
- [ ] 3.3 若场景暴露真实缺口，在本任务内修码至绿；harness 全量过

## 4. 对拍收尾

- [ ] 4.1 [blocked-by: 2.3; blocked-by: 3.3] 真实 attach 手测迁移清单 §4 步骤 6：`--session` 长历史一次出全文、无假 spinner
- [ ] 4.2 勾根 `_TUI_MIGRATED_TODO.md` P0「A1 快照」项；P0 清零后核对是否满足收口条件（P1 未勾仍挡删文件）
