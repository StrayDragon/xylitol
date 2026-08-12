---
depends_on:
- c1755-update-tui-travel-notice-placement
- c2070-add-package-tui-dual-interaction-modes
blocks:
- c2050-update-activity-fold-mouse-leader
branch: sdd/c1760-add-tui-activity-fold
base_sha: 839990764660a6b9f9014d1a88ee2d35aafeccd6
checkpointed: false
---

# TUI activity-fold — 多级折叠（含 Worked for）

> **一句话**：长会话中间操作墙多级折叠（L2 计数摘要 → L3 `Worked for …`），可配置降级；键盘双向栈；鼠标点击后置。

> 纯 UI；≠ `domain-compaction`。主 ROI = 长历史 / resume 冷 paint。前置 c1755 / c2070 已归档；L1 三角+per-id 已由 c2040 归档交付——本 change **不重做** L1，只与之分层共存。

## Why

长会话中间 Tool/Thinking/Bash 墙过高；resume/重建冷 paint 重。需要对齐 Cursor 式 **多级折叠**：细账 → 活动计数摘要 → 最粗通用耗时表达，在可读性与少画之间可配置降级。不宣称治流式尾。

## What Changes

1. **段级状态** `SegmentLevel ∈ {L0, L2, L3}`（无段级「L1」——L1 = 块级 `ScrollbackFold`，正交）。
2. **M2+C1**：`activity.expandNearest` / `activity.collapseNearest`；TUI 默认 `Alt+Shift+E` / `Ctrl+Alt+Shift+E`（可改绑）。
3. **L2/L3 摘要行**：`▸/▾`（Ascii `>/v`，对齐 c2040）+ 满和弦旁注；render 维护 `segment_id → [line_start, line_end)`（鼠标预留，本波不接线点击）。
4. **配置降级**：`keep_recent_turns`（默认 2）；rebuild / turn-end 自动压到 L2（默认开）；远段自动 L3（默认关）。
5. **L3 wall clock**：可靠两端戳才写 `Worked for …`；否则停 L2 / 省略时长——**禁止伪造**。

## Capabilities

| capability | 角色 |
|---|---|
| `app-tui-transcript` | 段 level、L2/L3 行、与 L1 分层、paint 局部失效 |
| `app-tui-host`（MAY） | 键位动作接线（若 host 路由需扩） |
| `runtime-config`（MAY） | activity_fold 配置面 |

## 已拍板（推荐默认 = 合约意向）

| 项 | 决定 |
|---|---|
| MVP 模型 | **M2** 段状态 + **C1** 双向栈；无块焦点、不抢 Editor |
| 段阶梯 | **L0 ↔ L2 ↔ L3**（一级步进）；块级 L1 仅在段=L0 时生效 |
| 同键跨级 | **共用** expand/collapse 一对和弦；**不**为 L3↔L2 另开第二和弦 |
| 展开 | `activity.expandNearest` = 最近 **L2/L3** 升一级（朝 L0） |
| 收纳 | `activity.collapseNearest` = 最近 **L0 Activity** 降一级；地板=该段默认粗级（未开远段 L3 → L2） |
| 无目标 | 两键均**静默** |
| 自动折叠 | rebuild **开**；直播回合**结束**后 auto→L2 **开**（流式中禁止压当前回合） |
| 窗口 | `keep_recent_turns` 默认 **2** |
| 远段自动 L3 | 可配，**默认关** |
| L3 时长 | 段 wall clock；缺戳 → L2 或无时长字段；**MUST NOT** 假时长 |
| L2 `+/-` | **有可靠 Diff 统计才显示**；否则省略；禁止伪造 |
| 旁注 | **满和弦**（形态 A）；L2/L3 行写 `(Alt+Shift+E)` / `(Ctrl+Alt+Shift+E)`，**不**写 `(Alt+E)` |
| 标记 | `▸/▾` / `>/v`（合流 c2040；不做 `(+)/(-)`） |
| 与 L1（c2040） | 段 L2/L3：**不进入**段内块 render；`Alt+E`/`Ctrl+T`/`Ctrl+O` 与 per-id overrides **不改变**该段外观。段 L0：既有 default+overrides 照常 |
| 鼠标 | **本波不实现点击**；MUST 预留行距缝。点击 → c2050（可被 c2045 吸收） |
| 旧 turn 外显 | User + 最终 Assistant；中间操作 ≥L2；ScrollNotice/Error **始终外显**（travel 见 c1755） |
| Compaction | ActivityFold **不吞** Compaction 块 |
| 跨面 | 动作语义统一（M1b）；物理键/点击分面 |
| 弱终端第二默认 | **只靠**用户 `keybindings.json`（不另发官方第二默认） |

## 与邻接 change 边界

| id | 本 change | 对方 |
|---|---|---|
| **c2040**（已归档） | 不重做三角/overrides/字形 | L1 Tool/Diff/Ask/Thinking per-id + hit |
| **c2045**（draft） | 不交付 Bash/Compaction/viewport 点折；不建广义 FoldTarget 总装 | 剩余可折块 + 可吸收 c2050 |
| **c2050** | `blocks`；只预留 segment↔行映射 | 段摘要鼠标 / 与 L1 目标模型统一 |
| **c1505 / c1370 / c1535** | 少画可缓解紧迫性；**不**实现切片/热缓冲/wrap | 性能并列候补 |

```text
Wave: c2070✓ → c2040✓ → [本 c1760] → c2050（或 c2045 吸收）
并行性能: c1505 / c1370 / c1535（非硬依赖）
```

## Out of scope

- 改 LLM 上下文 / session compaction 算法
- 假 `Worked for` / 假 `+/-`
- keyboard fold-leader / 数字编号（已废弃）
- scrollback 常驻焦点槽；本波鼠标点击
- 统一全部 System/ScrollNotice 文案族（另案）
- 实现 c1505 / c1370 / c1535 / c2045

## Open Questions

| # | 题 | 决议 |
|---|---|---|
| 1 | 弱终端官方第二默认？ | **已钉**：只靠用户 json |
| 2 | 旁注形态？ | **已钉**：满和弦 A |
| 3 | L3↔L2 是否第二和弦？ | **已钉**：否，共用 C1 一对键 |
| 4 | 直播回合结束 auto？ | **已钉**：默认开（仅 ended turn） |
| 5 | L2 文案语言？ | **已钉**：英文模板（Cursor 体）；i18n 另案 |
| 6 | 时间戳进 UI？ | **已钉**：rebuild/live 须能读段边界时钟（见 design）；丢戳则不 L3 |
| 7 | c2050 vs c2045 吸收？ | **已钉（与 c2045/c2050 对齐）**：段命中由 **c2045 吸收**；c2050 → docs-only。本波只留行距缝，**不**交付段鼠标 |

## Ethics

- risk_level: low–medium
- prohibited_actions: 静默丢细账不可逆；L3 伪造时长；fold 冒充 session compaction；偷渡 delayed 性能实现；本波抢 Editor 常驻焦点；把 `Alt+E` 绑成段级展开；本波交付段鼠标点击冒充 c2050
- required_evidence: 多级行数/局部 paint；双向栈可逆；与 L1 overrides 分层（apply 前）
- escalation_policy: 改 JSONL/模型上下文 → 用户确认

## Further Notes

- 交互深挖（分层表、跨面、三修饰风险）收口在 [`design.md`](./design.md)。
- 同源板：`docs/roadmaps/Web与TUI同源.md` M1b。
- c2040 归档：`archive/2026-08-12-c2040-add-tui-mouse-click-fold-triangle/`（L1 不得重复）。
- c2050：[`../c2050-update-activity-fold-mouse-leader/proposal.md`](../c2050-update-activity-fold-mouse-leader/proposal.md)；c2045 可吸收评估保留在对方。

## Start readiness

| 项 | 值 |
|---|---|
| **ready_for_start** | **yes** |
| 剩余人决 | **无挡 start**。可选（不挡）：L2 英文计数模板细词（files/searches/commands）在 Specs landing 时按 Cursor 体定稿即可 |
| 下一步 | 干净树 + 默认分支 → `llman sdd change start c1760-add-tui-activity-fold` → Specs landing（见 `tasks.md` §1） |
| 禁止 | 本阶段改 `llmanspec/specs/**` / 应用代码；勿 `attach` 到 main |
