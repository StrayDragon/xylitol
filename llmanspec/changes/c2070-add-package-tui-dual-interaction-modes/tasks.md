# Tasks: c2070-add-package-tui-dual-interaction-modes

> 验收一致：每条 task 完成 = 对应 seam 绿 + 不越界进 `blocks`（fold / viewport slice）。
> **人验（2026-08-11）**：`just demo-tui-alt-screen` — 滚轮 sticky、退出 dump、dock 夹边续选手感 **PASS**。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 1–2 升格 / Specs landing | ✅ | — |
| 3 生命周期 / 视口 / dump | ✅ | ptim01–02, 09–11 |
| 4 transcript 选区 / dock | ✅（含人验） | ptim03–07, 12 |
| 5 Editor 独立多行选区 | ✅ | ptim13 |
| 6 复制成功短提示 | ✅ 库+demo+产品 ath31 | **ptim15** + ath31 |
| 7 库 host seam / 收口 | ⬜ 部分 | ptim14、ath30 |

---

## 1. 升格、调研与规划壳 — ✅

- [x] 1.1 升格 + cascade `depends_on`/`blocks`
- [x] 1.2 Pi / Zellij / subsystem-cut 调研
- [x] 1.3 proposal / design / tasks 钉「完整 Mode B 库基础」

## 2. Branch binding 与 Specs landing — ✅

- [x] 2.1 attach `sdd/c2070-…`
- [x] 2.2 live `package-tui-interaction-modes` + `ath30`
- [x] 2.3 合约扩至 ptim01–ptim15、ath30–ath31（本轮补 ptim15/ath31）
- [x] 2.4 validate 结构门禁（pending tasks 期间 `--strict` 会报未勾任务，属预期）

## 3. 库：生命周期与视口 — ✅

- [x] 3.1 Mode A/B API + alt begin/end
- [x] 3.2 复用 c2020 mouse；Moved 不刷帧
- [x] 3.3 ScrollView + ModeBRuntime；滚轮 sticky follow（**人验 PASS**）
- [x] 3.4 suspend/resume 重进 alt+mouse（ptim11）
- [x] 3.5 退出 dump 主屏 scrollback（**人验 PASS**）

## 4. 库：transcript 选区 / dock — ✅

- [x] 4.1 拖选高亮 + 松手 OSC52 默认开
- [x] 4.2 越界续选 + idle tick
- [x] 4.3 按下始于 dock 不启 transcript 选区（ptim06）
- [x] 4.4 双击词 / 三击行 SHOULD；fold hit 钩子预留
- [x] 4.5 拖选中进 dock：夹底边续选、不清选（ptim12，**人验 PASS**）

## 5. 库：Editor 独立选区 — ✅

- [x] 5.1 Editor（或共享缓冲选区类型）未修饰拖选，覆盖**多行**缓冲（ptim13）
- [x] 5.2 高亮仅输入可视行；松手复制仅输入文本；与 transcript `SelectionController` 隔离
- [x] 5.3 Mode B：按下始于 dock → 事件回落 Editor；transcript 选区可清；包级单测
- [ ] 5.4 `just demo-tui-alt-screen` 人验 Editor 多行选区

**验收**：包单测 + demo 拖选输入多行 → 高亮 → 松手 OSC52；不影响 transcript 选区状态机。

## 6. 复制成功短提示 — ✅（人验 H7 可选）

- [x] 6.1 库：松手复制成功后发出可观察 **copy-notice** 信号（pending flag / callback / 等价），空选或不复制 MUST NOT 发（ptim15）
- [x] 6.2 库/demo：短时 UI 提示（TTL 约 1.5–3s）；落点优先 **dock 内、输入框上方 1 行**（或 demo 等价），MUST NOT 写入 transcript / ScrollNotice
- [x] 6.3 产品 host：Mode B 下将 copy-notice 接到壳层短提示（ath31）；**MUST NOT** 滥用 `Error: ` 前缀的 chrome-toast 拒闸形态冒充成功确认（可用独立 info 槽或扩展非 Error toast——design 钉落点）
- [x] 6.4 单测：复制成功 → notice 置位/清除；人验 demo 可见「已复制」类短文案（单测 ✅；人验 H7 待）

**验收文案（建议固定）**：`Copied` / `已复制`（中英择一钉死在 design；demo 与产品同源）。

## 7. 产品闸与库 host 收口 — ⬜/部分

- [x] 7.1 Host `TuiRunOptions.interaction_mode`；默认 Mode A；换栈；dock 登记；不读 `XYLITOL_TUI_MOUSE`（ath30）
- [x] 7.2 包测 + host harness；`demo-tui-alt-screen` 人验路径
- [ ] 7.3 AGENTS / package 文档：Mode B **下游接入清单**（ptim14）
- [ ] 7.4 `llman sdd validate --strict` 全绿；verify 双轴无 CRITICAL；确认未实现 `c1760`/`c2040`/`c2050`/`c1505`/`c1535`
- [ ] 7.5（可选）产品显式 Mode B 开关文档化（仍默认 A）

---

## 人验清单（`just demo-tui-alt-screen`）

| # | 项 | 状态 |
|---|---|---|
| H1 | alt-buffer 进入/退出 | ✅ |
| H2 | transcript 拖选高亮 + 松手 OSC52 | ✅ |
| H3 | 滚轮 sticky（非仅靠边续选） | ✅ |
| H4 | 拖选进输入区夹边续选、不反选 | ✅ |
| H5 | 退出后主屏可上翻会话 dump | ✅ |
| H6 | Editor 多行独立选区 | ⬜ 高亮 PASS；边沿自动滚已修，待复验 |
| H7 | 复制成功短提示 | ✅ |

## 实现顺序（建议）

```text
6.1–6.2（库 copy-notice + demo 提示）
  → 5.1–5.4（Editor 选区）
  → 6.3（产品 ath31）
  → 7.3–7.4（文档 + verify 收口）
```
