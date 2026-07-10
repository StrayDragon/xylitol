# _HANDOFF — xylitol TUI（交接笔记，非规范）

> 最后更新：2026-07-10（轨 A 收口；产品面仍 **冻结**；c491 stub-only）
> 分支：`feat/tui-dev`（ahead origin；含 c457/c458 等）
> **本文是临时交接/进度板，不是 SSOT。** 稳定边界以各层 `AGENTS.md` 与 skills 为准。

---

## 〇、三轨并行

| 轨 | 目标 | 状态 |
|---|---|---|
| **A. 包 / agent_demo** | demo 验证原子 | **主线已收口**（至 c458 / c471） |
| **B. 产品面** | `src/app/tui` ↔ Driver / XyEvent | **冻结**：c460 空壳 + c491 假树 stub |
| **D. DESIGN** | 主 DESIGN + `design/*.md` | **已落地**（c449）；可改文档，不据此堆产品实现 |

**闸门**：用户明确开闸 → 才继续轨 B（bridge / input / chrome / 垂直切片）。c491 **MUST NOT** 在 stub 上扩展。

**原则**：demo 用真实库验证 → 再搬产品面；缺能力先改包；purpose-draft 升格后才 apply。

---

## 一、已锁定产品决议

| 主题 | 决议 |
|---|---|
| Esc | 流中 = `Driver::abort`（**清 steer，保留 follow_up** 供 UI restore — c461 D4） |
| Ctrl+C | 有输入→清编辑器；空→退出 |
| 流中 Enter | **steer** |
| Alt+Enter | **follow-up** |
| 审批 | trust 后 yolo；保留 hooks |
| Status | idle **0 行** |
| Expandable | 仅应用面（demo/产品）；包有 `ExpandableOutput` 辅助 |
| Diff | `package-tui-diff`；word-level；宽屏可 L/R |
| Slash MVP | `/exit` + `/model` |
| Feature | `tui` 在 **default** |
| 远控 | 默认 InProcess；保留 RemoteDriver |
| 高亮 | demo/产品同一真实 syntect；包只收回调 |
| 日志 | debug 默认即时 log；release 默认关 |
| 会话树 | demo 活树 SSOT；产品 c491 **stub 冻结** |
| Bash / `$EDITOR` | demo：TTY 真编辑器 + harness stub；产品 c492 后置 |
| Theme auto | demo c458 opt-in；产品 MVP **固定暗色** |

---

## 二、Change DAG（当前）

### 轨 A — 已归档（节选）

```text
c449 DESIGN split ✓
c451 Diff ✓ · c452 highlight ✓ · c453 conditional ✓
c454 TreeSelector ✓ · c455 InputListener ✓ · c456 nav keys ✓
c457 bash/Ctrl+G stub ✓ · c458 theme auto ✓
c459–c464 / c466–c469 / c471（diff/tool-bg/expandable/tree/steer/pan/fork）✓
```

### 轨 B — 活跃 purpose-draft（冻结中，勿 apply）

```text
c460 host ✓ archived
c461 steer/follow-up seam ✓ archived
c491 session-tree stub ✓ archived（stub-only）

c465 bridge          ← 开闸后首选
c475 chrome          ← 依赖 c449✓
c480 input           ← 依赖 c455✓ / c461✓
c485 vertical slice  ← c465 + c475 + c480
c470 transcript      ← paused（不做 Codex TranscriptView）
后置: c490 trust · c492 bash · c493 compaction/retry
```

- 图：`llman sdd graph --scope active --format mermaid`
- specs 层前缀 rename：`_tmp_prompts/SPECS_RENAME_MAP.md`（若仍在）

---

## 三、下一步

1. **开闸决策**：升格 / apply **c465**（XyEvent→UI bridge）——轨 B 依赖链起点。
2. 并行候选（仍需开闸）：**c480** input/slash（依赖已齐）。
3. 冻结期内允许：c460/c491 harness 回归、文档 / AGENTS / `_HANDOFF`、包内通用缺口（非产品接线）。
4. 空壳可 `cargo run -- --tui`（需 TTY）；真聊一轮等 c485。

---

## 四、`agent_demo` 键位（应对齐产品）

| 键 | 作用 |
|---|---|
| 流中 Enter | steer |
| Alt+Enter | follow-up |
| Esc | abort（清 steer / 留 follow_up）；经 InputListener |
| Ctrl+C | 清输入 / 空则退 |
| 双 Esc | 会话树；Enter travel（+reply spine）；Shift+F fork |
| `!` 前缀 | bash 边框 |
| Ctrl+G | `$EDITOR`（TTY 真路径 / harness stub） |
| `/` `@` · Ctrl+T/Alt+E/Alt+G · Ctrl+P/S · Ctrl+O | 见 seed 系统行 |
| `XYLITOL_AGENT_DEMO_THEME_AUTO=1` | 可选亮暗探测 |

---

## 五、SSOT 指针

- 边界：`packages/xylitol-tui/AGENTS.md`、`src/app/tui/AGENTS.md`、`src/AGENTS.md`、根 `AGENTS.md`
- vs pi：`packages/xylitol-tui/PI_DELTAS.md`
- How-to：`write-tui`、`test-tui-harness`、`write-surface`
- 视觉：`src/app/tui/DESIGN.md` + `design/`
- 合约：`llmanspec/changes/archive/2026-07-10-c450-…`、`…/c461-…`
