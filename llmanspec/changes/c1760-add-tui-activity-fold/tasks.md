# Tasks: c1760-add-tui-activity-fold

> **前置**：c1755 / c2070 / c2040 已归档。决策见 `proposal.md`（Open Questions 全钉或显式推迟）；细节见 `design.md`。
> **硬禁**：Apply 勿抢跑 c2045/c2050 段鼠标 / 广义 FoldTarget；勿占用 att29+（预留给 c2045）。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 0 Designed 规划壳 | ✅ | proposal + design + tasks |
| 1 Specs landing | ✅ | att23–att28 + feature:false unit |
| 2–7 Apply | ✅ | Apply backlog |

---

## 0. Designed（本波文档）— ✅

- [x] 0.1 充实 `proposal.md`：已拍板 + 邻接边界 + Open Questions 钉死 + Start readiness
- [x] 0.2 写 `design.md`：L0–L3、L1 分层、键盘、配置、性能、c2045/c2050
- [x] 0.3 写本 `tasks.md`（Specs 预备 + Apply backlog）

---

## 1. Specs landing — ✅

- [x] 1.1 Branch binding：`sdd/c1760-add-tui-activity-fold` 已 attach
- [x] 1.2 `app-tui-transcript`：att23–att26（段 level、L2/L3、与 L1 分层、自动降级窗口）
- [x] 1.3 键位：att28 `activity.expandNearest` / `collapseNearest`；无目标静默（产品级；未另开 host req）
- [x] 1.4 `runtime-config`：不强制 YAML MUST；defaults 写进 att26 code-first
- [x] 1.5 场景：att23–att28 各一条 `feature: false` unit（对齐 att19–22）
- [x] 1.6 确认 **不**写 Segment 鼠标点击 / `FoldTarget::Segment` 为 MUST（att23 显式 MUST NOT；属 c2050/c2045）
- [x] 1.7 commit Specs landing → `readyToImplement=true`

---

## Apply backlog

### 2. 段模型 + 时钟

- [x] 2.1 定义 Activity 段边界与稳定 `segment_id`（live ≡ rebuild）
- [x] 2.2 段边界时钟：rebuild/live 可读 wall-clock 两端（旁路戳或 session 树直算）
- [x] 2.3 `SegmentLevel` 状态存 UI root（或等价）；与 `ScrollbackFold` 分离

### 3. 自动降级 + 配置

- [x] 3.1 `keep_recent_turns`（默认 2）：近窗 L0
- [x] 3.2 rebuild auto → 超窗段 L2（默认开）
- [x] 3.3 turn-end auto → 超窗段 L2（默认开；流式当前回合不压）
- [x] 3.4 `auto_l3_distant` 默认关；开时地板可为 L3
- [x] 3.5 `activity_fold.enabled` 总开关

### 4. L2/L3 render

- [x] 4.1 L2 计数摘要行 + 可选 `+/-`（无可靠 Diff 则省略）
- [x] 4.2 L3 `Worked for …`；缺戳降级
- [x] 4.3 标记 `▸/▾`（Ascii `>/v`）；满和弦旁注跟绑定
- [x] 4.4 User + 最终 Assistant 外显；ScrollNotice/Error/Compaction 不进收纳
- [x] 4.5 维护 `segment_id → [line_start, line_end)` 预留缝（不接线 fold hit）

### 5. 键盘 C1 + 与 L1 分层

- [x] 5.1 注册 `app.activity.expandNearest` / `collapseNearest` 默认和弦
- [x] 5.2 expand/collapse 一级步进；无目标静默；地板规则
- [x] 5.3 段 L2/L3 时 Alt+E / Ctrl+T / Ctrl+O / L1 三角不影响该段外观
- [x] 5.4 升到 L0 后 c2040 overrides/三角恢复有效

### 6. 验证

- [x] 6.1 harness：自动 L2、双向栈、静默、分层、缺戳不假 L3
- [x] 6.2 paint：段 level 切换局部失效（无全历史 MD 重解析）
- [x] 6.3 `just fmt` + 相关测；人验最短路径记 verify 板

### 7. 收口

- [x] 7.1 `llman sdd validate c1760-add-tui-activity-fold --strict`
- [x] 7.2 确认未实现 c2045 剩余块点折 / c2050 段点击 / c1505·c1370·c1535
- [x] 7.3（可选）同源板 M1b 动作 id 与本文一致的文档句指针

## 实现顺序

```text
0 Designed ✅
  → 1 Specs landing ✅ → readyToImplement
  → 2 段模型/时钟 → 3 配置/自动 → 4 render/缝 → 5 键位/分层 → 6 harness → 7 validate
```

## Open Questions

见 `proposal.md`——无未决项挡 apply；c2050↔c2045 吸收显式推迟到对方 propose。
