# Tasks: c2070-add-package-tui-dual-interaction-modes

> 验收一致：每条 task 完成 = 对应 seam 绿 + 不越界进 `blocks`（fold / viewport slice / **产品 ath30**）。
> **人验（2026-08-11）**：`just demo-tui-alt-screen` — 滚轮 sticky、退出 dump、dock 夹边续选手感 **PASS**。
> **范围钉（2026-08-12）**：本分支 **仅** 库基础极致化（双入口、ptim14、API 质量）。**产品 B-only / ath30** → [`c2071`](../c2071-update-app-tui-host-mode-b-only/)。

## 进度总览

| 块 | 状态 | 合约 |
|---|---|---|
| 1–2 升格 / Specs landing | ✅ | — |
| 3 生命周期 / 视口 / dump | ✅ | ptim01–02, 09–11 |
| 4 transcript 选区 / dock | ✅（含人验） | ptim03–07, 12 |
| 5 Editor 独立多行选区 | ✅ | ptim13 |
| 6 复制成功短提示 | ✅ 库+demo+产品 ath31 | **ptim15** + ath31 |
| 7 库双入口 / host seam 收口 | ⬜ **本分支主战场** | ptim14、优雅 API、结构债 |

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
- [x] 5.4 `just demo-tui-alt-screen` 人验 Editor 多行选区（单击无鬼影 PASS 2026-08-12；边沿自动滚已裁）

**验收**：包单测 + demo 拖选输入多行 → 高亮 → 松手 OSC52；不影响 transcript 选区状态机。

## 6. 复制成功短提示 — ✅（人验 H7 可选）

- [x] 6.1 库：松手复制成功后发出可观察 **copy-notice** 信号（pending flag / callback / 等价），空选或不复制 MUST NOT 发（ptim15）
- [x] 6.2 库/demo：短时 UI 提示（TTL 约 1.5–3s）；落点优先 **dock 内、输入框上方 1 行**（或 demo 等价），MUST NOT 写入 transcript / ScrollNotice
- [x] 6.3 产品 host：Mode B 下将 copy-notice 接到壳层短提示（ath31）；**MUST NOT** 滥用 `Error: ` 前缀的 chrome-toast 拒闸形态冒充成功确认（可用独立 info 槽或扩展非 Error toast——design 钉落点）
- [x] 6.4 单测：复制成功 → notice 置位/清除；人验 demo 可见「已复制」类短文案（单测 ✅；人验 H7 待）

**验收文案（建议固定）**：`Copied` / `已复制`（库信号 + demo 可先用英文）。**产品落点 / 误触策略**：延后到 `src/app/tui` Mode B 集成（[`c2071`](../c2071-update-app-tui-host-mode-b-only/)）再钉；本 change 不阻塞。

## 7. 库双入口 / host seam 收口 — ⬜（本分支主战场）

> **已迁出**：原 **7.5 产品 ath30 B-only** → [`c2071`](../c2071-update-app-tui-host-mode-b-only/)。本块只做库。

- [x] 7.1 Host 换栈 / dock API 已存在（历史默认 A；产品默认翻转属 c2071）
- [x] 7.2 包测 + host harness；Mode B 人验路径（env demo 过渡已废；两 example）
- [ ] 7.3 **ptim14 库 host 接入清单**（AGENTS / package）：dock→Editor origin→mouse 单一路径；copy-notice 臂装；**产品可抄、禁止 demo 私有胶为 SSOT**
- [ ] 7.4 `llman sdd validate --strict`；verify 双轴无 CRITICAL；确认未实现 blocks / 未改 ath30 产品默认
- [x] 7.6 **Demo 拆分**：`agent_demo`（Inline）与 `agent_demo_alt`（Mode B）两文件 + `agent_demo_impl`；just 两 recipe；删除 `XYLITOL_AGENT_DEMO_MODE`
- [x] 7.7 **Mode B e2e**（PTY）：alt/mouse/dump、OSC52 拖选、wheel smoke、dock clamp copy、suspend/resume；tmux Mode B 粗烟仍 SHOULD
- [ ] 7.8 **双入口结构极致化**（本分支核心）：
  - 收敛 `application_session_active` 散布；抽出 ApplicationOwned 会话/生命周期（或等价 type-state / 分治 facade）
  - teardown：`finish_application_owned` vs `finish_inline`（命名与行为对齐）
  - dump opt-out API（ptim02 MAY）
  - 统一 Editor 命中契约（一条 remap 路径 + 包测）
  - 面向扩展：fold hit 优先级钩子可挂；零多余运行时分支税；接口面小而稳（pub 边界审一遍）
- [ ] 7.9 （可选）包 docs / examples 展示「优美 host 最小环」：`begin` → `dispatch` → `idle_tick` → `finish`，无 demo 特例泄漏

---

## 人验清单（Mode B demo）

| # | 项 | 状态 |
|---|---|---|
| H1 | alt-buffer 进入/退出 | ✅ |
| H2 | transcript 拖选高亮 + 松手 OSC52 | ✅ |
| H3 | 滚轮 sticky（非仅靠边续选） | ✅ |
| H4 | 拖选进输入区夹边续选、不反选 | ✅ |
| H5 | 退出后主屏可上翻会话 dump | ✅ |
| H6 | Editor 多行独立选区 | ✅ 2026-08-12（边沿滚已裁） |
| H7 | 复制成功短提示 | ✅ demo；产品落点 → c2071 |

## 实现顺序（2026-08-12 修订²）

```text
战略：产品 B-only → c2071（本分支不碰 ath30）
  → 7.8 双入口结构 / 优雅 API（主）
  → 7.3 ptim14 库 host 清单（与 7.8 可交错）
  → 7.9 最小 host 环示例（可选）
  → 7.4 validate / verify
  → archive c2070 → 另开分支做 c2071
```
