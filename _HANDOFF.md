# _HANDOFF — 交接板 + 短索引（非规范）

> 最后更新：2026-07-12（**c575 已归档**；无活跃 change；轨 B 至 c493）
> 分支语境：`feat/tui-dev`（相对 `origin/feat/tui-dev` 超前）
> **临时交接 / 进度指针，不是 SSOT。** 稳定边界：各层 `AGENTS.md`、`docs/architecture/`、`llmanspec/`。

---

## 〇、去哪读（短索引）

| 主题 | 去哪 |
|---|---|
| 产品定调 / Trust·MCP / Provider 范围 | 根 + `src/AGENTS.md` |
| **全部产品架构图（唯一入口）** | [`docs/architecture/README.md`](docs/architecture/README.md) |
| 队列运行时实现 | archive **c525** `design.md` |
| XyEvent 防宽表实现 | archive **c520**；产品摘要见架构目录 |
| TUI bridge / chrome / input / slice | archive **c465** … **c493**；合约 `app-tui-*` / `app-tui-vertical-slice` |
| 包 Overlay focus-restore（D08） | archive **c575**；`package-tui-engine` pte06 |
| 日常满闸 | **`just qa`**（见根 `AGENTS.md`） |
| 真终端满闸 | **`just qa-e2e`**；产品 Fake smoke：`just test-tui-e2e-pty` |
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

# 产品 PTY Fake smoke（#[ignore]）
just test-tui-e2e-pty
```

---

## 一、现状一览

| 轨 | 范围 | 状态 |
|---|---|---|
| **P · 包 / demo / DESIGN** | `packages/xylitol-tui`、`agent_demo`、`DESIGN.md`+`design/*` | **已合入**（包侧至 **c575** 已归档） |
| **A · 业务核心** | `domain`→`embed`/`server`/线协议 | **已归档**（c500–c525 + 业务侧 c530–c550） |
| **B · 产品 TUI** | `src/app/tui` 接线 | **至 c493 已归档**；c491 stub 冻结 |

**号段注意**：轨 P 与轨 A 曾并行占用 **c530–c550**；以 `llmanspec/changes/archive/` **全名**为准。

**原则**

1. 缺通用 TUI 能力 → 先改 `packages/xylitol-tui`（可先 `agent_demo` 验证），再进产品面。
2. 产品面只经 `Driver` / `dispatch` / `XyEvent`；不 reach `agent`/`infra` 内部。
3. **c491 假树 stub** 仍冻结扩展（仅双 Esc / Esc 关 / Enter `travel → id`）；真活树另 change。

### 近期已落地（本分支）

| 项 | 说明 | 状态 |
|---|---|---|
| c465–c482 | bridge · chrome · live scrollback · trust · input · history · abort-resume | 已归档 |
| **c485** vertical slice | `ScriptedDriver` H1–H9 + 产品 PTY Fake（Hello → `/exit`） | 已归档 |
| **c492** bang-bash | `!`/`!!` → `execute_bash` + 边框 + Ctrl+G stub | 已归档 |
| **c493** compaction/retry | Compacting / Retry 单行 status；End 恢复 Working | 已归档 |
| **c575** overlay focus-restore | eligible/blocked/resume + dispatch reclaim（D08） | 已归档 |
| secret.env + YAML 模板 | `{{ secret.KEY }}` / `{{ env.KEY }}` | 已合入 |
| `just qa` / `qa-e2e` | 统一日常 / 真终端满闸（c580） | 已归档 |

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
| **c575** | Overlay eligible/blocked/resume focus-restore（D08） |
| 跟进提交（无独立 change） | 弱终端 Markdown 强调；窄宽 clamp；Atoms plates；死代码分诊 |

**活跃 change**：无。

---

## 四、轨 B — 产品 TUI（当前主线）

```text
已归档：… · c485 slice · c492 bash · c493 compaction/retry
包侧：  c575 overlay focus-restore 已归档
冻结：  c491 假树 stub（勿扩活树）
已移除：c470 Codex TranscriptView
```

开闸记录：`src/AGENTS.md` / `src/app/tui/AGENTS.md`。视觉闸：`src/app/tui/DESIGN.md`。

### 剩余 change 简报

无活跃 SDD change。

### 下一工作焦点

1. 另开产品 follow-up（`/compact` slash、真 emit AutoRetry、活树等），或
2. 整理 `feat/tui-dev` PR / 合并。

---

## 五、已锁定产品决议（demo / 产品应对齐）

| 主题 | 决议 |
|---|---|
| Esc | 流中 = abort（清 steer，留 follow_up）；abort 后可继续聊（c482） |
| Ctrl+C | 有输入→清编辑器；空→退出 |
| 流中 Enter / Alt+Enter | steer / follow-up |
| Status | idle **0 行** |
| Diff | word-level；宽屏可 L/R；SBS 无行底 |
| Slash MVP | `/exit` + `/model` |
| 会话树 | demo 活树；产品 **c491 stub**（勿在 stub 上扩活树） |
| Theme | 产品 MVP **固定暗色**；demo `/theme` + 可选 `THEME_AUTO` |
| 高亮 | demo/产品同一 syntect 回调；包只收回调 |
| Transcript | **不做** Codex TranscriptView（原 c470 已移除）；live = bridge scrollback |
| 垂直切片 | 合成 harness + 产品 PTY Fake smoke（c485 / `app-tui-vertical-slice`） |
| Bash | `!`/`!!` → `execute_bash`；busy 不另开 bash（c492） |
| Compaction UI | 单行 Compacting/Retry；End→Working（c493） |

---

## 六、验证速查

| 闸 | 命令 |
|---|---|
| 日常 | `just qa` |
| 真终端 | `just qa-e2e` |
| 包 TUI | `cargo test -p xylitol-tui` |
| Overlay focus | `cargo test -p xylitol-tui --test overlay_focus_test` |
