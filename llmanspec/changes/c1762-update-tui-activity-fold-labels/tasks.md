# Tasks: Activity 折叠诚实词表

> 决策见 `proposal.md`；树与算法见 `design.md`。硬禁：默认分支改 live specs；本票实现 c1770 snapshot；改 att34 切刀；L1 动词重命名。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 0 Designed 规划壳 | ✅ | proposal + design + tasks |
| 1 Specs landing | ✅ | att24 / att33 / att23 独簇句 |
| 2–5 Apply | ✅ | 计数 · paint · live · 夹具 |

---

## 0. Designed

- [x] 0.1 `proposal.md` 词表、XOR、Ran 大小写、+/-、与 c1761/c1770 边界
- [x] 0.2 `design.md` 树 + 簇头算法 + compaction 独簇 paint
- [x] 0.3 本 `tasks.md`
- [x] 0.4 `research/session-460ad16e-fold-honesty.md`

## 1. Specs landing

- [x] 1.1 干净树且默认分支：`llman sdd change start c1762-update-tui-activity-fold-labels`
- [x] 1.2 `app-tui-transcript`：改写 att24（诚实类目、XOR、去重、basename、Title Case、禁止 file 占位）；att33 进行时 Editing/Exploring/Running；att23 补「仅 compaction 不套簇头」
- [x] 1.3 对应 `feature: false` unit 场景；禁止 toon `feature: true`
- [x] 1.4 commit Specs landing → `readyToImplement=true`

## Apply

### 2. 计数与格式

- [x] 2.1 删除 `files = 1` 占位；MCP/未知不进 files
- [x] 2.2 Edited XOR Explored；Ran 第二子句；N 去重 + basename；回退 Thought / Used / Asking
- [x] 2.3 +/- 仍 ToolEnd 可靠 diff；无则省略
- [x] 2.4 单测：thinking-only ≠ Explored；compaction-only 无 Explored；MCP → Used；edit+read 只 Edited；bash-only → Ran

### 3. Paint

- [x] 3.1 仅 Compaction 的簇不画簇头、不登记簇 hit；信封展开直出 compaction 块
- [x] 3.2 信封 L3 仍收纳 compaction（回归 att23）
- [x] 3.3 视觉 `design/activity-fold.md` 文案与 playground 槽对齐新词表（无滑入）

### 4. Live window

- [x] 4.1 打开簇进行时：Editing / Exploring / Running；封口改过去式
- [x] 4.2 更新 `live_tape` 期望；禁止纯 read 显示 Editing
- [x] 4.3 live 默认折叠子项（不弹流式正文）；Thought 合并 thinking L1；paint 不每帧改折叠态
- [x] 4.4 Thought 时长：live 用本流第一次 ThinkingDelta 的墙钟；flush 后冻在该 Thinking id；resume 无戳则省略，禁止伪造

### 5. 夹具与闸

- [x] 5.1 `/debug activity-fold-live` / `activity-fold-resume` 断言跟新词表
- [x] 5.2 `just fmt` + 相关 `activity_fold` / harness 测
- [x] 5.3 手测指针：session `460ad16e`「cool 总结下」（不进 qa）；`/debug activity-fold-live`：Planning → Thought（无 `thinking (Ctrl+T)`、默不弹正文、≥1s 见 `Thought Ns`）→ Exploring 簇头默认折叠 → 点三角见 Read → 助手正文始终外显 → 下一提问后上一轮 `Worked for`

## 收口

- [x] 6.1 `llman sdd validate c1762-update-tui-activity-fold-labels --strict --no-check`
- [x] 6.2 确认未改 att34、未接线 c1770、未改 L1 块动词
