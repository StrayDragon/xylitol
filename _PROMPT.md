# 主线 Agent Prompt（Track B · 产品 TUI）

> **用法**：新会话把下面「可粘贴 Prompt」整段发给 Agent。本文与 `_HANDOFF.md` 同步维护；**非规范**。
> SSOT：根/`src`/`src/app/tui` `AGENTS.md`、`docs/architecture/`、`llmanspec/changes/`。
> 分支语境（2026-07-11）：`feat/tui-dev`（轨 A + 轨 P 已合入；相对 `main` 超前）。

---

## 可粘贴 Prompt

```text
你在 xylitol（Rust 2024 / llman SDD）仓库工作。用中文回复。先读真值，再动手。

## 当前主线（唯一焦点）

Track B · 产品 TUI：`src/app/tui/`
- 下一刀：**apply `c465-add-app-tui-bridge`**（XyEvent→UI 单缝 + host 合流 `Driver::run` EventStream + QueueUpdate）
- 建议顺序：c465 →（升格）c475 chrome / c480 input → c485 垂直切片
- **paused**：c470 Codex TranscriptView（永久不做本 id）
- **后置**：c490 trust · c492 bash · c493 compaction UI
- **可选不阻塞**：包侧 c575 Overlay focus-restore（purpose-draft）

提案/设计：`llmanspec/changes/c465-add-app-tui-bridge/{proposal,design,tasks}.md`
技能：先 `/llman-sdd-apply`（或等价 skill）；产品面写法 `write-tui`；包测 `test-tui-harness`。

## 已收口（不要重做 / 不要当未完成）

- Track A：c500–c525 + 业务侧 c530–c550（embed / Server→Driver / 线协议 / MCP seam / dispatch）
- Track P：包侧 c530–c570 已合入；`xylitol-tui` = 源自 pi-tui 的 **独立 fork**（见 `packages/xylitol-tui/NOTICE`），按 xylitol 需求分叉，不追平上游
- 产品 TUI **已开闸**；host 空壳 c460 + c491 **假树 stub** 已落地

## 硬约束

1. 分层：`app → agent → runtime_protocol → domain`；应用面只经 `Driver` / `dispatch` / `XyEvent`；禁止 reach `agent::session` / `runtime` / `infra`。
2. 缺通用 TUI 能力 → 先 `packages/xylitol-tui`（可先 `agent_demo`），再进 `src/app/tui`。
3. **禁止**在 c491 stub 上扩活树 / filter / 真 Driver travel。
4. Provider Pre-1.0：仅 OpenAI 兼容 + Anthropic。
5. 提交 Conventional Commits；不加 co-author / agent 身份。
6. 改动聚焦；实现后同步 llmanspec tasks 勾选；开 PR 前 `just qa`。
7. 号段 c530–c550 轨 A/P 曾冲突 → 以 archive **全名**为准。

## 必读（短）

- `_HANDOFF.md`（进度板）· `_NOTE.md`（短索引）
- `src/AGENTS.md` · `src/app/tui/AGENTS.md` · `packages/xylitol-tui/AGENTS.md`
- `docs/architecture/README.md`（产品语义；非代码 SSOT）
- 键位/视觉决议：`src/app/tui/DESIGN.md` + `design/keybindings.md`

## 本会话默认任务

1. 确认 working tree / 分支后，按 `llman-sdd-apply` 推进 **c465**（或用户指定的下一 Track B change）。
2. 每完成一刀：更新 tasks 勾选；需要时一句更新 `_HANDOFF.md` 轨 B 状态；不要把进度抄进 AGENTS。
3. 不确定目标时先问用户；不要并行铺 c475/c480/c485 大骨架，除非用户明确要求。
4. 验证：相关单测 / harness；涉及包则 `cargo test -p xylitol-tui`；收尾倾向 `just lint` 或用户要求的 `just qa`。

开始前用 3–5 行复述：你理解的下一刀、会改哪些路径、不会碰什么。然后执行。
```

---

## 进度怎么跟

| 时机 | 做什么 |
|---|---|
| 会话开始 | 读 `_HANDOFF.md` §〇 + §三；看 `llmanspec/changes/` 未归档项 |
| apply 中 | 勾 `tasks.md`；代码真值优先于本文 |
| 归档后 | `/llman-sdd-archive`；把 `_HANDOFF` 轨 B「下一」指针前移一格 |
| 包侧小打磨 | 仅当阻塞产品面；否则保持 Track B |

## 一句话现状

轨 A / 轨 P 已合入 → **主线 = Track B，入口 c465** → 目标垂直切片 c485；c491 stub 与 c470 paused 不动。
