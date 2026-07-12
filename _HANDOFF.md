# _HANDOFF — 交接板 + 短索引（非规范）

> 最后更新：2026-07-12（c580 统一 just qa / qa-e2e；轨 B 下一 c475/c480）
> 分支语境：`feat/tui-dev`（相对 `main` 超前）
> **临时交接 / 进度指针，不是 SSOT。** 稳定边界：各层 `AGENTS.md`、`docs/architecture/`、`llmanspec/`。
> 原 `_NOTE.md` 内容已并入本文；`_NOTE.md` 仅作跳转 stub。

---

## 〇、去哪读（短索引）

| 主题 | 去哪 |
|---|---|
| 产品定调 / Trust·MCP / Provider 范围 | 根 + `src/AGENTS.md` |
| **全部产品架构图（唯一入口）** | [`docs/architecture/README.md`](docs/architecture/README.md) |
| 队列运行时实现 | archive **c525** `design.md` |
| XyEvent 防宽表实现 | archive **c520**；产品摘要见架构目录 |
| TUI bridge（轨 B · 已归档） | archive **`2026-07-12-c465-add-app-tui-bridge`** |
| 日常满闸 | **`just qa`**（fmt+clippy+test+test-tui+doc+tokens+prek） |
| 真终端满闸 | **`just qa-e2e`**（qa + PTY/tmux 第 5 层） |
| 产品视觉 MUST（唯一） | `src/app/tui/DESIGN.md` + `design/*.md` |
| **产品 TUI 活实验场** | `just demo-tui`（`packages/xylitol-tui/examples/agent_demo.rs`） |
| 浏览器静图 | `src/app/tui/design/playground/`（`sync_tokens.py`） |
| 包边界 / vs pi | `packages/xylitol-tui/AGENTS.md`、`PI_DELTAS.md` |

勿在本文件复制架构长文；只留指针与进度。

### 预览（人类）

```bash
# 活实验场（产品 TUI 快速 playground）
just demo-tui

# 浏览器静图（token / 整壳）
just open-design-playground

# 改色板后
just sync-tui-tokens && just check-tui-tokens
```

---

## 一、现状一览

| 轨 | 范围 | 状态 |
|---|---|---|
| **P · 包 / demo / DESIGN** | `packages/xylitol-tui`、`agent_demo`、`DESIGN.md`+`design/*` | **已合入**（包侧 c530…c570 已归档） |
| **A · 业务核心** | `domain`→`embed`/`server`/线协议 | **已归档**（c500–c525 + 业务侧 c530–c550） |
| **B · 产品 TUI** | `src/app/tui` 接线 | **c465 已归档**；下一升格 **c475 chrome / c480 input → c485** |

**号段注意**：轨 P 与轨 A 曾并行占用 **c530–c550**；以 `llmanspec/changes/archive/` **全名**为准。

**原则**

1. 缺通用 TUI 能力 → 先改 `packages/xylitol-tui`（可先 `agent_demo` 验证），再进产品面。
2. 产品面只经 `Driver` / `dispatch` / `XyEvent`；不 reach `agent`/`infra` 内部。
3. **c491 假树 stub** 仍冻结扩展（仅双 Esc / Esc 关 / Enter `travel → id`）；真活树另 change。

### 近期已落地（本分支，相对交接）

| 项 | 说明 | commit / 状态 |
|---|---|---|
| c465 bridge | `apply_xy_event` + host `select!` 合流 EventStream | `8fec5a9`（已归档） |
| secret.env + YAML 模板 | `{{ secret.KEY }}` / `{{ env.KEY }}` | `19dd0c4` · `adfa669` |
| TUI trust | `interactive: true` + stdio trust prompt | 同上批 |
| playground / DESIGN 组织 | **单份** app DESIGN；`agent_demo`=产品活实验场；删包侧 HTML playground | **未 commit** |

---

## 二、轨 A — 业务缝（已收口）

| 波次 | Change（全名摘要） | 说明 |
|---|---|---|
| A0–A1 | c520 · c500 · c510 · c505 | XyEvent 闭集；精选导出；domain 去 JsonSchema；Provider 单路径 |
| A1′ | c525 · c515 | 异步队列 + QueueUpdate；MCP 配置化重载 |
| A2 | c530 embed · c535 Server→Driver · c540 线协议/Remote | 公开嵌入 API；Server 持 Driver；`Event::QueueUpdate` + Remote REST |
| A3 | c545 MCP seam · c550 Server dispatch | 嵌入侧 MCP 规格去泄漏；REST 经 `dispatch(Command)` |

路径：`llmanspec/changes/archive/2026-07-11-c5xx-*`（业务全名；勿与同号段包侧 archive 混淆）。
产品语义：`docs/architecture/`。

---

## 三、轨 P — 包打磨（已收口）

| Change | 主题 |
|---|---|
| c530 … c570（package-tui / demo / playground） | Markdown · Command plate · Diff 边角 · CompletionSource · Expandable · DESIGN sync · TreeSelector · ChoicePrompt · Palette/`/theme` |
| 跟进提交（无独立 change） | 弱终端 Markdown 强调；窄宽 clamp；Atoms plates；死代码分诊 |

**可选下一刀（purpose-draft，不阻塞轨 B）**：[`c575-add-package-tui-overlay-focus-restore`](llmanspec/changes/c575-add-package-tui-overlay-focus-restore/)（`PI_DELTAS` D08）。

---

## 四、轨 B — 产品 TUI（当前主线）

```text
已归档：c460 host · c461 队列 seam · c491 stub-only · c465 bridge
下一：  c475 chrome / c480 input → c485 垂直切片
paused：c470 Codex TranscriptView（不做）
后置：  c490 trust UI · c492 bash · c493 compaction/retry UI
可选：  c575 overlay focus-restore（包侧，非轨 B 阻塞）
```

开闸记录：`src/AGENTS.md` / `src/app/tui/AGENTS.md`。

### 剩余 change 简报

| ID | 状态 | 做什么 |
|---|---|---|
| **c475** chrome | purpose-draft · **下一刀** | 语义 token→闭包主题；glyph；idle **0 行** status；busy 一行；footer=`cwd · model`；产品 MVP **固定暗色** |
| **c480** input | purpose-draft · **下一刀** | Editor 区；`/exit` `/model`；流中 Enter=steer / Alt+Enter=follow-up / Esc=abort / Ctrl+C 清或退；双 Esc→**c491 stub** |
| **c485** vertical slice | purpose-draft · MVP 门槛 | TTY→提交→流式/工具→steer/follow-up→abort→`/exit` 可聊一轮 E2E（依赖 c475+c480） |
| **c470** transcript | **paused** | 不做 Codex 式 TranscriptView；浏览改双 Esc 树 |
| **c490** trust UI | purpose-draft · 后置/可并行 | 未信任时 TUI 选择器（优先 editor 槽）；stdio trust 已有，本 change 补 **壳内 UX** |
| **c492** bash | purpose-draft · 后置 | `!` bash 边框 + `Driver::execute_bash`；输出进 live scrollback |
| **c493** compaction/retry | purpose-draft · 后置 | Compaction / AutoRetry 的 status·scrollback 呈现 |
| **c575** overlay restore | purpose-draft · 包侧可选 | Overlay 完整 focus-restore（不阻塞轨 B） |

**建议顺序**：DESIGN（已钉 c475/c480 MUST + playground）→ 升格 apply **c475** → **c480** → **c485**。

### 下一工作焦点

1. 升格 `c475-add-app-tui-chrome`（specs/tasks）并 apply。
2. 升格 `c480-add-app-tui-input` 并 apply。
3. `just open-design-playground` / `just demo-tui` 对照验收。

---

## 五、已锁定产品决议（demo / 产品应对齐）

| 主题 | 决议 |
|---|---|
| Esc | 流中 = abort（清 steer，留 follow_up） |
| Ctrl+C | 有输入→清编辑器；空→退出 |
| 流中 Enter / Alt+Enter | steer / follow-up |
| Status | idle **0 行** |
| Diff | word-level；宽屏可 L/R；SBS 无行底 |
| Slash MVP | `/exit` + `/model`（产品接线随 c480） |
| 会话树 | demo 活树；产品 **c491 stub**（勿在 stub 上扩活树） |
| Theme | 产品 MVP **固定暗色**；demo `/theme` + 可选 `THEME_AUTO` |
| 高亮 | demo/产品同一 syntect 回调；包只收回调 |
| Transcript | **不做** Codex TranscriptView（c470 paused）；live = bridge scrollback |

### `agent_demo` 键位（摘要）

| 键 / 命令 | 作用 |
|---|---|
| 流中 Enter / Alt+Enter | steer / follow-up |
| Esc | abort |
| Ctrl+C | 清输入 / 空则退 |
| 双 Esc | 会话树 |
| `!` / Ctrl+G | bash 边框 / `$EDITOR` |
| `/theme [dark\|light\|toggle]` | 显式换肤（关 auto） |
| `XYLITOL_AGENT_DEMO_THEME_AUTO=1` | COLORFGBG 探测 |

---

## 六、已清理的文档（历史）

- 删除：`docs/testing-strategy.md`、`docs/tui-research/*`（**保留** `docs/assets/logo.svg`）
- 测试分层要点已并入根 `AGENTS.md`「提交与测试」
- `_NOTE.md` → stub，内容并入本文件（2026-07-12）

---

## 七、SSOT 指针

| 主题 | 路径 |
|---|---|
| 分层 / 导出 / 开闸 | 根 + `src/AGENTS.md`、`src/app/tui/AGENTS.md` |
| 产品架构图 | `docs/architecture/` |
| 包边界 / vs pi | `packages/xylitol-tui/AGENTS.md`、`PI_DELTAS.md` |
| 视觉 | `src/app/tui/DESIGN.md` + `design/` |
| How-to | `write-tui`、`test-tui-harness`、`write-surface`、`audit-dead-code` |
| 交接 / 短索引 | 本文件 `_HANDOFF.md` |
