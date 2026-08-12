# Tasks: c2040-add-tui-mouse-click-fold-triangle

> **前置**：c2020 / c2070 / c2071 已归档。深挖决策见 `proposal.md` Open Questions（全钉）。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 1 Specs landing | ⬜ | att* / ath* 点折 + per-id |
| 2 覆盖表 + ThinkingId | ⬜ | tools/thinking default+overrides |
| 3 Hit 表 + host 接线 | ⬜ | 三角列；A+latch |
| 4 字形 ▸/▾ | ⬜ | width=1；ascii |
| 5 Harness / 单测 | ⬜ | seam 1–5 |
| 6 validate / 收口 | ⬜ | ready→apply 后 verify |

---

## 1. Specs landing — ⬜

- [ ] 1.1 Branch binding：`change start`（`sdd/c2040-…`）
- [ ] 1.2 `app-tui-transcript`：per-block overrides、Thinking per-id、三角列点折、全局清族 overrides、字形
- [ ] 1.3 `app-tui-host`：AO 下接线 `set_transcript_hit_priority`；拖选 latch；折叠点击进范围（新 ath，不破坏 ath29/30 既有句）
- [ ] 1.4 场景：`feature: false` unit（harness 覆盖）；避免无 step 的裸 `.feature` 挂死 check
- [ ] 1.5 commit Specs landing → `readyToImplement`

## 2. 覆盖表 + ThinkingId — ⬜

- [ ] 2.1 `ScrollbackFold`（或等价）：tools overrides map + thinking overrides map
- [ ] 2.2 Thinking 条目稳定 id（live + rebuild 同构）
- [ ] 2.3 `Alt+E` / `Ctrl+T`：flip default + 清对应族 overrides
- [ ] 2.4 effective 展开态驱动 render（Tool/Diff/Ask/Thinking）

## 3. Hit 表 + host 接线 — ⬜

- [ ] 3.1 render 维护三角列 `fold_hit_regions`（绑 paint gen）
- [ ] 3.2 `set_transcript_hit_priority`：命中 → toggle + 吞按
- [ ] 3.3 拖选进行中忽略 fold 重命中（latch）
- [ ] 3.4 点正文 / 旁注不 toggle

## 4. 字形 — ⬜

- [ ] 4.1 Unicode fold/unfold → `▸`/`▾`；Ascii 保持 `>`/`v`
- [ ] 4.2 `visible_width==1` 单测；ascii env 冒烟

## 5. 验证 — ⬜

- [ ] 5.1 harness：Mouse 三角 toggle / 误点正文 / 全局清覆盖
- [ ] 5.2 ath25 miss 上界不因单块 toggle 回退
- [ ] 5.3 `just fmt` + 相关测；人验最短路径记 verify 板

## 6. 收口 — ⬜

- [ ] 6.1 `llman sdd validate c2040 --strict`
- [ ] 6.2 确认未实现 c2045 / c1760 / c2050 范围

## 实现顺序

```text
1 Specs landing → 2 覆盖表+ThinkingId → 3 hit 接线 → 4 字形 → 5 harness → 6 validate
```

## Open Questions（已钉）

见 `proposal.md`；无未决项挡 apply。
