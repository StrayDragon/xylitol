# Verify — c1955-add-agent-todo-subsystem

**Date:** 2026-08-12
**Branch:** `sdd/c1955-add-agent-todo-subsystem`
**Worktree:** `/home/l8ng/Projects/__straydragon__/xylitol.sdd-c1955-add-agent-todo-subsystem`
**base_sha (proposal):** `839990764660a6b9f9014d1a88ee2d35aafeccd6`
**HEAD (this verify):** `3da893bb88533ea053ecd2a90c9e986b76b8bba8`
**Stage:** `full` · `readyToImplement=true` · `specsLanded=true` · attached
**CARGO_TARGET_DIR:** per-worktree via `eval "$(just cargo-wt-env)"`

## Gates

| Gate | Result |
|---|---|
| Branch binding | `sdd/c1955-add-agent-todo-subsystem` |
| `llman sdd show … --type change --output json` | `readyToImplement=true` / `specsLanded=true` / `stage=full` |
| `llman sdd validate c1955… --strict --no-check` | **PASS**（INFO: design.md present） |
| `cargo test --lib todo` | **16/16 PASS** |
| `cargo test --lib` filters: `builtin_concurrency_class_table` / `default_tools_include_todo` | **PASS** |
| `cargo test --test bdd` | **276/276 PASS**（含 `test_tools_all_ten_smoke` / `test_tools_output_overflow`） |
| Footer 长路径假失败 | **本 worktree 未复现**（见下；**非 CRITICAL**） |

### Footer / 长路径说明（环境）

Worktree 绝对路径较长。BDD `output-overflow`（`test_tools_output_overflow`）断言 `[Full output:` / `lines shown` 与截断体量；在本树 **PASS**，未出现因路径嵌入 footer 导致的字节/断言假失败。
**判定：footer 长路径问题 ≠ CRITICAL**（环境风险留意项；与 c1955 合约无关，且本轮未触发）。

---

## 合约轴（Spec）

对照 live SSOT：`llmanspec/specs/agent-todo/spec.toon`（atd1–12）+ `agent-tools`（r42 / t2 / t26）；diff = `83999076...HEAD`。

### CRITICAL

（无）

### Covered requirements

| Req | Verdict | Evidence |
|---|---|---|
| **atd1** domain model | PASS | `protocol/session/todo.rs`：`TodoItem`/`TodoStatus` 闭集；空表合法；`validate_todo_list`；单测 `domain_status_closed_set_roundtrip` / `empty_content_rejects` |
| **atd2** Custom latest-wins | PASS | `CUSTOM_TYPE_AGENT_TODO` + `latest_agent_todo` 全叶扫描；`SessionAgentTodoGateway` append；单测 `latest_wins_full_scan` |
| **atd3** 不进 LLM 前缀 | PASS | `agent_todo_custom_skipped_in_llm_prefix`；Custom（非 CustomMessage） |
| **atd4** 三工具语义 | PASS | `infra/tools/todo.rs` list/rewrite/update；未知 id 拒绝无快照；`rewrite_returns_full_list_and_persists` / `todo_list_readonly` / BDD `all-ten-tools-smoke` |
| **atd5** ≤1 in_progress | PASS | domain + tool 双层 `dual_in_progress_rejects` |
| **atd6** Barrier | PASS | `execution_mode` Sequential；`builtin_concurrency_class_table` |
| **atd7** 栏只读边界 | PASS | 本 diff **无**栏注入 / Agent 列 publish / 栏→SSOT 写回；Todo 仅经 gateway→Custom |
| **atd8** 可折 checklist | PASS | `UiEntry::Todo` + 默认折叠摘要 `Todo · d/t`；`todo_checklist_defaults_to_summary_line` / expand；无 Plan 侧栏 |
| **atd9** resume / tool 刷新 | PASS | leaf rebuild → `sync_todo_checklist_from_entries`；`ToolExecutionEnd` → `sync_todo_checklist_from_tool_result`；driver `bind_todo_session`；Print 无 UI、gateway 同源 |
| **atd10** compact 重挂 | PASS | `ensure_agent_todo_after_compact` + `compact_reappends_agent_todo_when_cut_drops_snapshot` |
| **atd11** 首轮冻表 | PASS | `freeze_includes_todo_builtins_first_turn` |
| **atd12** export 标记 | PASS* | `session_export`：`[custom:agent_todo]` 且 `header` 块（非用户 message）—*实现明确；缺专用单测见 WARNING* |
| **r42 / t2** 闭集 10 工具 | PASS | `default_tools_include_todo_closed_set`；BDD `@req:r42` `all-ten-tools-smoke`；feature/spec 文案已改「10」 |
| **t26** todo_* Barrier | PASS | 同 atd6 / `builtin-class-table` 意向 |

### Out-of-scope（正确未做）

- Agent 状态栏 / Lane Runtime / Agent 列（c1895/c1896）
- 用户手改 checklist、`/todo` slash、max-attempt、子 agent、即时 plan

### WARNING（合约轴）

1. **atd12 缺专用导出单测** — HTML/JSONL 路径已实现标记，但无 `session_export` 定向断言；不挡 archive（代码审查 + 既有 export 回归绿）。
2. **atd9 无独立 TUI harness「resume→checklist」用例** — 接线在 `session_tree` / driver bind；折叠渲染有单测。完整 PTY resume 可后补。

---

## 标准轴（Standards）

权威：`AGENTS.md` / `src/AGENTS.md`（分层：`protocol` 值类型+port → `infra::tools` TypedTool → `agent` 薄协作/compact → `app` 组合根+TUI）。

### CRITICAL

（无）

### WARNING

1. **`default_tools()` 默认 MemoryTodoGateway** — 产品路径经 `default_tools_with_todo(SessionAgentTodoGateway)`；裸 `default_tools()` 测/烟测可落内存。文档/调用方需知悉，避免误当已落盘。
2. **architecture 词表「Todo 清单」未改** — tasks 标可选跳过；chrome 词汇台账可后补。

### SUGGESTION

1. 为 `session_export` 加一条 `agent_todo` → `[custom:agent_todo]` 且非 `message` 块的单测（钉 atd12）。
2. Memory vs Session gateway 的 list/rewrite/update 错误映射已集中在 port 实现；保持勿在 TUI 再解析第二套状态机。

### 坏味（判断性，非硬违规）

- **Duplicated Code（轻）**：MemoryTodoGateway 与 Session 路径均走同一 `normalize_*` / `apply_todo_update` — 可接受。
- **Speculative Generality**：未见为子 agent/namespace 预埋抽象 — 符合 D4。

---

## Verdict

**合约轴：通过（CRITICAL=0）。标准轴：通过（CRITICAL=0）。**

| 项 | 值 |
|---|---|
| Verdict | **PASS** |
| CRITICAL | **0** |
| Footer 长路径假失败 | **非 CRITICAL**（本轮未复现） |
| HEAD sha | `3da893bb88533ea053ecd2a90c9e986b76b8bba8` |
| 可 archive？ | **YES** |

下一步：`llman-sdd-archive` / `change finalize`（本报告 **不** inline finalize；勿 push）。
