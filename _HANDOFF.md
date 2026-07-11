# _HANDOFF — 交接板（非规范）

> 最后更新：2026-07-11（轨 P 已合入当前分支；轨 A 内核缝归档；轨 B 已开闸）
> 分支语境：`feat/tui-dev`（含轨 A 业务缝 + 轨 P 包打磨；相对 `main` 超前）
> **临时交接 / 进度指针，不是 SSOT。** 稳定边界：各层 `AGENTS.md`、`docs/architecture/`、`llmanspec/`。

---

## 〇、现状一览

| 轨 | 范围 | 状态 |
|---|---|---|
| **P · 包 / demo / DESIGN** | `packages/xylitol-tui`、`agent_demo`、`DESIGN.md`+`design/*` | **本批完成并已合入**（包侧 c530…c570 已归档） |
| **A · 业务核心** | `domain`→`embed`/`server`/线协议 | **已归档**（c500–c525 + 业务侧 c530–c550） |
| **B · 产品 TUI** | `src/app/tui` 接线 | **已开闸**；下一入口 **`c465` bridge** |

**号段注意**：轨 P 与轨 A 曾并行占用 **c530–c550**；以 `llmanspec/changes/archive/` **全名**为准（如 `c530-update-package-tui-markdown` vs `c530-add-lib-embed-api`）。

**原则**

1. 缺通用 TUI 能力 → 先改 `packages/xylitol-tui`（可先 `agent_demo` 验证），再进产品面。
2. 产品面只经 `Driver` / `dispatch` / `XyEvent`；不 reach `agent`/`infra` 内部。
3. **c491 假树 stub** 仍冻结扩展（仅双 Esc / Esc 关 / Enter `travel → id`）；真活树另 change。

---

## 一、轨 A — 业务缝（已收口）

| 波次 | Change（全名摘要） | 说明 |
|---|---|---|
| A0–A1 | c520 · c500 · c510 · c505 | XyEvent 闭集；精选导出；domain 去 JsonSchema；Provider 单路径 |
| A1′ | c525 · c515 | 异步队列 + QueueUpdate；MCP 配置化重载 |
| A2 | c530 embed · c535 Server→Driver · c540 线协议/Remote | 公开嵌入 API；Server 持 Driver；`Event::QueueUpdate` + Remote REST |
| A3 | c545 MCP seam · c550 Server dispatch | 嵌入侧 MCP 规格去泄漏；REST 经 `dispatch(Command)` |

产品语义：`docs/architecture/`。短索引：`_NOTE.md`。

---

## 二、轨 P — 包打磨（已收口）

| Change | 主题 |
|---|---|
| c530 … c570（package-tui / demo / playground） | Markdown · Command plate · Diff 边角 · CompletionSource · Expandable · DESIGN sync · TreeSelector · ChoicePrompt · Palette/`/theme` |
| 跟进提交（无独立 change） | 弱终端 Markdown 强调；窄宽 clamp；Atoms plates；死代码分诊（见 `e9e6f92`） |

**可选下一刀（purpose-draft，不阻塞轨 B）**：[`c575-add-package-tui-overlay-focus-restore`](llmanspec/changes/c575-add-package-tui-overlay-focus-restore/)（`PI_DELTAS` D08）。

---

## 三、轨 B — 产品 TUI（当前主线）

```text
已归档：c460 host · c461 队列 seam · c491 stub-only
下一：  c465 bridge → c475 chrome / c480 input → c485 垂直切片
paused：c470 Codex TranscriptView（不做）
后置：  c490 trust · c492 bash · c493 compaction/retry UI
```

开闸记录：`src/AGENTS.md` / `src/app/tui/AGENTS.md`。提案：`llmanspec/changes/c465-add-app-tui-bridge/`。

---

## 四、已锁定产品决议（demo / 产品应对齐）

| 主题 | 决议 |
|---|---|
| Esc | 流中 = abort（清 steer，留 follow_up） |
| Ctrl+C | 有输入→清编辑器；空→退出 |
| 流中 Enter / Alt+Enter | steer / follow-up |
| Status | idle **0 行** |
| Diff | word-level；宽屏可 L/R；SBS 无行底 |
| Slash MVP | `/exit` + `/model`（产品接线随轨 B） |
| 会话树 | demo 活树；产品 **c491 stub**（勿在 stub 上扩活树） |
| Theme | 产品 MVP **固定暗色**；demo `/theme` + 可选 `THEME_AUTO`（COLORFGBG only） |
| 高亮 | demo/产品同一 syntect 回调；包只收回调 |

### `agent_demo` 键位（摘要）

| 键 / 命令 | 作用 |
|---|---|
| 流中 Enter / Alt+Enter | steer / follow-up |
| Esc | abort |
| Ctrl+C | 清输入 / 空则退 |
| 双 Esc | 会话树 |
| `!` / Ctrl+G | bash 边框 / `$EDITOR` |
| `/theme [dark\|light\|toggle]` | 显式换肤（关 auto） |
| `XYLITOL_AGENT_DEMO_THEME_AUTO=1` | COLORFGBG 探测（勿写 OSC 进 crossterm 环） |

---

## 五、SSOT 指针

| 主题 | 路径 |
|---|---|
| 分层 / 导出 / 开闸 | 根 + `src/AGENTS.md`、`src/app/tui/AGENTS.md` |
| 产品架构图 | `docs/architecture/` |
| 包边界 / vs pi | `packages/xylitol-tui/AGENTS.md`、`PI_DELTAS.md` |
| 视觉 | `src/app/tui/DESIGN.md` + `design/` |
| How-to | `write-tui`、`test-tui-harness`、`write-surface`、`audit-dead-code` |
| 轨 A 短索引 | `_NOTE.md` |
| **主线 Agent Prompt** | `_PROMPT.md`（新会话粘贴用） |
