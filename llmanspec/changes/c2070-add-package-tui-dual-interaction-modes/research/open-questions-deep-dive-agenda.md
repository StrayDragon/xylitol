# Open questions / 深挖调研建议（战略修订后）

> 2026-08-12。人拍：产品 B-only；库双入口分治；demo 两文件；高自动化 e2e。
> 完整矩阵以 [`mode-b-e2e-automation-and-open-questions.md`](./mode-b-e2e-automation-and-open-questions.md)（agent 产出）为准；本文件给**优先深挖题**便于人拍与开 research ticket。

## P0 — 挡产品 B-only / 拆 demo

| # | 问题 | 为何深挖 | 建议产出 |
|---|---|---|---|
| Q1 | 双入口物理形状：两 `Tui*` 类型（Pi）vs 共享引擎+策略对象？ | 今日单 `TUI` + `application_session_active` 散布；分叉功能会继续缠 | ADR 短文 + 迁移步序（tasks 7.8） |
| Q2 | `ath30` 改写边界：产品 MUST B-only 后，库 `ptim01`/`ptim08` 如何措辞？ | 避免「产品无 A」被读成「库删 A」 | Specs landing 草案（勿直接改 live 前对齐） |
| Q3 | Demo 拆分时 e2e 探针 / `DEMO_READY_NEEDLE` / just recipe 如何双挂？ | 现 e2e 绑 `agent_demo`；拆文件会断闸 | 迁移清单 + 最小双 recipe |

## P1 — 挡「敢默认 B」信心

| # | 问题 | 为何深挖 | 建议产出 |
|---|---|---|---|
| Q4 | Mode B 行为自动化 MUST 集 vs 人验-only 集？ | 拖选/dump/dock/Editor 今日偏人验 | 行为×harness 矩阵（pty/tmux/unit） |
| Q5 | tmux / SSH / Ghostty / Wez 哪些进 CI、哪些 nightly、哪些永人验？ | Pi fullscreen 标 experimental 有终端税 | 环境矩阵 + 签字人 |
| Q6 | 退出 dump 是否足以替代「会话中终端 scrollback」对核心用户？ | B-only 去掉会话中原生历史上滚 | 短用户研究或内部试用笔记 |

## P2 — fold 前 / 结构债

| # | 问题 | 为何深挖 | 建议产出 |
|---|---|---|---|
| Q7 | 点击命中优先级：fold triangle > OSC8 > selection 的库 hook 形状？ | c2040 硬前置；避免再缠选区 | 接口草图（实现仍属 c2040） |
| Q8 | Inline 入口长期：仅 example+测，还是保留「文档化逃生」？ | 产品无 UI 切 A 后仍可能要 debug | 政策一行钉死 |
| Q9 | Copied 落点与误触检测（产品） | 已延后；集成时再开 | 独立小 explore |

## 建议 research tickets（短标题）

1. **Dual-entry cut plan** — Pi 两 class vs xylitol 策略对象迁移步序
2. **ath30 B-only wording** — 产品 MUST B、库仍暴露 Inline 的合约句式
3. **Demo split + e2e rebind** — 两 example 与 `tests/tui_e2e` 探针
4. **Mode B PTY mouse protocol** — 自动化拖选/滚轮在 PTY 的可行边界
5. **tmux Mode B matrix** — 与 Pi terminal-setup 对照的 CI 子集
6. **Fold hit-test seam** — 预留优先级钩子，不实现 fold

## 与进行中 agent 产出

- Strict review → `mode-b-strict-review-2026-08-12.md`
- E2E / open Q → `mode-b-e2e-automation-and-open-questions.md`

落地后把冲突处回写 `proposal.md` Further Notes。
