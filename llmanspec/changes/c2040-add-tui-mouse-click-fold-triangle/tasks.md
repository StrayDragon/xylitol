# Tasks: c2040-add-tui-mouse-click-fold-triangle

> **前置**：c2020 / c2070 / c2071 已归档。深挖决策见 `proposal.md` Open Questions（全钉）。
> **门禁**：`readyToImplement=true`（Specs landed）。下方 Apply backlog 由 `llman-sdd-apply` 实施时改回 checkbox 并勾选。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 1 Specs landing | ✅ | att19–22 / ath33 |
| 2–6 实现 | ✅ | 见 Apply backlog |

---

## 1. Specs landing — ✅

- [x] 1.1 Branch binding：`change start`（`sdd/c2040-…`）
- [x] 1.2 `app-tui-transcript`：att19–22（字形 / tools overrides / thinking per-id / 三角列）
- [x] 1.3 `app-tui-host`：ath33 hit_priority + latch
- [x] 1.4 场景：`feature: false` unit
- [x] 1.5 commit Specs landing → `readyToImplement`

## Apply backlog（实施时勾选）

### 2. 覆盖表 + ThinkingId

- [x] 1. `ScrollbackFold`（或等价）：tools overrides map + thinking overrides map
- [x] 2. Thinking 条目稳定 id（live + rebuild 同构）
- [x] 3. `Alt+E` / `Ctrl+T`：flip default + 清对应族 overrides
- [x] 4. effective 展开态驱动 render（Tool/Diff/Ask/Thinking）

### 3. Hit 表 + host 接线

- [x] 1. render 维护三角列 `fold_hit_regions`（绑 paint gen）
- [x] 2. `set_transcript_hit_priority`：命中 → toggle + 吞按
- [x] 3. 拖选进行中忽略 fold 重命中（latch）
- [x] 4. 点正文 / 旁注不 toggle

### 4. 字形

- [x] 1. Unicode fold/unfold → `▸`/`▾`；Ascii 保持 `>`/`v`
- [x] 2. `visible_width==1` 单测；ascii env 冒烟

### 5. 验证

- [x] 1. harness：Mouse 三角 toggle / 误点正文 / 全局清覆盖
- [x] 1b. harness：拖选划过三角不 toggle；Thinking/Diff/Ask 三角 Mouse toggle；库 `hit_priority` 仅 Left Down
- [x] 2. ath25 miss 上界不因单块 toggle 回退
- [x] 3. `just fmt` + 相关测；人验最短路径记 verify 板（人验 PASS）

### 6. 收口

- [x] 1. `llman sdd validate c2040 --strict`
- [x] 2. 确认未实现 c2045 / c1760 / c2050 范围

## 实现顺序

```text
1 Specs landing ✅ → 2 覆盖表+ThinkingId → 3 hit 接线 → 4 字形 → 5 harness → 6 validate
```

## Open Questions（已钉）

见 `proposal.md`；无未决项挡 apply。
