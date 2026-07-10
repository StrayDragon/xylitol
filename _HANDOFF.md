# _HANDOFF — xylitol TUI（交接笔记，非规范）

> 最后更新：2026-07-10（c460 host archived）
> 分支：`feat/tui-dev`
> **本文是临时交接/进度板，不是 SSOT。** 稳定边界以各层 `AGENTS.md` 与 skills 为准。

---

## 〇、三轨并行

| 轨 | 目标 | 真值落点 |
|---|---|---|
| **A. 包 / agent_demo** | demo 验证原子（diff/高亮/树/InputListener/键位…） | `packages/xylitol-tui` + `agent_demo` + `PI_DELTAS.md` |
| **B. 产品面** | `src/app/tui` ↔ Driver / dispatch / XyEvent | `src/app/tui/` + `write-tui`；合约已归档 **c450** |
| **D. DESIGN** | 主 DESIGN + `design/*.md` | `src/app/tui/DESIGN.md`、`design/`（**c449** draft） |

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
| Expandable | 仅 `src/app/tui` |
| Diff | `package-tui-diff`；word-level；宽屏可 L/R |
| Slash MVP | `/exit` + `/model` |
| Feature | `tui` 在 **default** |
| 远控 | 默认 InProcess；保留 RemoteDriver |
| 高亮 | demo/产品同一真实 syntect；包只收回调 |
| 日志 | debug 默认即时 log（**代码待 c460**）；release 默认关 |

---

## 二、Change DAG（当前）

```text
c450 ✓ archived
c461 ✓ archived（steer/follow-up seam）
c455 ✓ archived（InputListener）
c460 ✓ archived（app-tui-host 空壳）
  c449 / c451 / c452 / c454 / c457 / c458 仍 purpose-draft

c460 host ← done
c465 bridge ← c460 + c461✓
c470 transcript ← c465 + c451 + c452
c475 chrome ← c460 + c449
c480 input ← c460 + c461✓ + c455✓
c485 slice ← c470 + c475 + c480
后置: c490–c493
```

- 图：`llman sdd graph --scope active --format mermaid`
- specs 层前缀 rename（P0+P1）已完成；映射表 `_tmp_prompts/SPECS_RENAME_MAP.md`

---

## 三、下一步（主线）

1. **升格 / apply c465**（XyEvent bridge）或 **c480**（input / slash）
2. 空壳已可 `cargo run -- --tui` 进入（需 TTY）
3. 已确认需求继续用 harness 验证（HostEvent / TestTerminal）

## 四、`agent_demo` 键位（应对齐产品）

| 键 | 作用 |
|---|---|
| 流中 Enter | steer（Driver 已就绪） |
| Alt+Enter | follow-up |
| Esc | abort（清 steer / 留 follow_up）；demo 经 InputListener |
| Ctrl+C | 清输入 / 空则退（InputListener） |
| 双 Esc | 会话树（c456） |
| `/` `@` · Ctrl+T/Alt+E/Alt+G · Ctrl+P/S · Ctrl+G | 见既有 demo |

---

## 五、SSOT 指针

- 边界：`packages/xylitol-tui/AGENTS.md`、`src/app/tui/AGENTS.md`、`src/AGENTS.md`、根 `AGENTS.md`
- vs pi：`packages/xylitol-tui/PI_DELTAS.md`（含 D16 InputListener）
- How-to：`write-tui`、`test-tui-harness`、`write-surface`
- 视觉：`src/app/tui/DESIGN.md` + `design/`（c449）
- 合约归档：`llmanspec/changes/archive/2026-07-10-c450-…`、`…/c461-…`
- 已归档：c450 / c455 / c460 / c461（见 `llmanspec/changes/archive/2026-07-10-*`）
