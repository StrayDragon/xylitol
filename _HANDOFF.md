# _HANDOFF — 双轨交接（非规范）

> 最后更新：2026-07-11（轨 P 本批收口）
> 分支语境：轨 A → `feat/tui-dev`；**轨 P** → worktree 分支 `polish/tui-components`（本批已 apply+archive，可开 PR）。
> **本文是临时交接/进度板，不是 SSOT。** 稳定边界：各层 `AGENTS.md`、`docs/architecture/`、`llmanspec/changes/`。

---

## 〇、三轨（2026-07-11 重命名对齐）

| 轨 | 范围 | 状态 | 冲突面 |
|---|---|---|---|
| **P · 包 / demo / DESIGN** | `packages/xylitol-tui`、`agent_demo`、可选 `src/app/tui/DESIGN.md`+`design/*`（只文档） | **本批提案队列已空**；可自由打磨或开 PR | 与轨 A 几乎零冲突（包零引用主 crate） |
| **A · 业务核心** | `src/{domain,runtime_protocol,agent,infra,app/core}`、c500–c525 | **可 apply**（任务未勾） | 主 crate；勿与轨 P 同改 `src/app/tui` 产品代码 |
| **B · 产品 TUI** | `src/app/tui` 接线、c465–c493 | **冻结** | 开闸 = 轨 A 相关落地 + 用户明确开闸 |

**原则**

1. 轨 P：组件化、可单独验证、每步可给人审；缺能力先改包，**禁止**在冻结期堆产品 bridge。
2. 轨 A：按波次 apply（见下）；Print 主线可回归。
3. 轨 B：purpose-draft 可改文档；**勿 apply** 直至开闸。

**Worktree 建议**

- 轨 P：`polish/tui-components`（当前；本批 done）
- 轨 A：留在 `feat/tui-dev`（或 `refactor/core-export`）
- Prompt 包：[`_prompts/track-p-tui-polish.md`](_prompts/track-p-tui-polish.md)（若缺文件，以本节 + `packages/xylitol-tui/AGENTS.md` 为准）

---

## 一、轨 A — 业务优化（本仓主线）

| 波次 | Change | 说明 |
|---|---|---|
| A0 | **c520** | XyEvent 闭集护栏（轻） |
| A1 | **c500** → **c510** | 精选 pub use + 删死包装 → domain 去 JsonSchema |
| A1′ | **c505** | Provider 单路径（可与 A1 并行） |
| A2 | **c525** | 异步队列 + QueueUpdate 单通道（**轨 B 开闸 P0**） |
| A3 | **c515** | MCP 配置化（可后置） |

产品语义图：`docs/architecture/`。实现只在对应 `llmanspec/changes/*/design.md`。

短索引：`_NOTE.md`。

---

## 二、轨 P — 包 / DESIGN / demo（worktree）

### 本批已收口（2026-07-11 · `polish/tui-components`）

| Change | 主题 | 归档 |
|---|---|---|
| c530 | Markdown token-efficient 渲染 | `archive/2026-07-11-c530-…` |
| c535 | agent_demo Command plate | `archive/2026-07-11-c535-…` |
| c540 | Diff CJK wrap + SBS 空半栏 | `archive/2026-07-11-c540-…` |
| c545 | CompletionSource + `$` 行内 | `archive/2026-07-11-c545-…` |
| c550 | Expandable Head hint / 零宽安全 | `archive/2026-07-11-c550-…` |
| c555 | DESIGN playground token sync + MD slot | `archive/2026-07-11-c555-…` |
| c560 | TreeSelector 空态 + 过滤选中稳定 | `archive/2026-07-11-c560-…` |

跟进（无独立 change）：Markdown 弱终端色强调（bold/italic）；有序嵌套列表 + 链接折行 fix。

**活跃 `llmanspec/changes/` 中已无轨 P 提案**；下一动作为开 PR、或下方自由打磨候选（先对齐再 propose）。

### 目标

把「人眼能审、机器能测、开闸能接」的原子做扎实：

1. **DESIGN**：token / 组件 MUST 可 HTML 预览（给人快速审色与层次）
2. **包组件**：单测 + snapshot；改一处验一处
3. **agent_demo**：只组合已验证原子；键位与产品决议对齐

### 工作方式（强制）

| 步 | 产出 | 人怎么审 | 机怎么验 |
|---|---|---|---|
| 1 | HTML token/组件预览页 | 浏览器看色板与块 | 静态文件即可 |
| 2 | 单组件改动 + harness | 看 demo 槽或 snapshot diff | `cargo test -p xylitol-tui` 相关 |
| 3 | demo 接线（若需要） | `just demo-tui` | 五层 harness 按需 |
| 4 | 短 PR / 短 commit | 对照 DESIGN MUST | `just qa` 子集 |

**禁止**：无预览/无单测就大改多组件；在 `src/app/tui` 实现产品 bridge。

### 自由打磨候选（未立项 · 需先对齐）

| 优先级感 | 候选 | 备注 |
|---|---|---|
| done | `MarkdownOptions` 死字段删除；list/table/quote 单次折行（`prewrapped`） | `polish/tui-components` 跟进 |
| done | SelectList/Settings/Loader/Input 窄宽 clamp + 空态 | SelectList 已有；Settings/Loader/Input 补齐 |
| done | playground Widgets 槽 + MD 列表格；demo plate `md-list-wrap` / `narrow-clamp` | 库用户运行时参考 |
| 低摩擦 | 补回 `_prompts/track-p-tui-polish.md`（链接现缺文件） | 可选 |

### 已锁定产品决议（demo 应对齐）

| 主题 | 决议 |
|---|---|
| Esc | 流中 = abort（清 steer，留 follow_up） |
| Ctrl+C | 有输入→清编辑器；空→退出 |
| 流中 Enter / Alt+Enter | steer / follow-up |
| Status | idle **0 行** |
| Diff | word-level；宽屏可 L/R；SBS 无行底 |
| Slash MVP | `/exit` + `/model`（产品开闸后） |
| 会话树 | demo 活树；产品 c491 **stub 冻结** |
| Theme | 产品 MVP **固定暗色**；demo 可 opt-in auto |
| 高亮 | demo/产品同一 syntect 回调；包只收回调 |

### `agent_demo` 键位（摘要）

| 键 | 作用 |
|---|---|
| 流中 Enter / Alt+Enter | steer / follow-up |
| Esc | abort |
| Ctrl+C | 清输入 / 空则退 |
| 双 Esc | 会话树 |
| `!` / Ctrl+G | bash 边框 / `$EDITOR` |
| `XYLITOL_AGENT_DEMO_THEME_AUTO=1` | 可选亮暗探测 |

---

## 三、轨 B — 产品 TUI（冻结）

```text
已归档：c460 host · c461 队列 seam · c491 stub-only
开闸后：c465 bridge → c475 chrome / c480 input → c485 垂直切片
paused：c470 Codex TranscriptView
后置：c490 trust · c492 bash · c493 compaction/retry
```

开闸前 P0（QueueUpdate 单通道）见 `llmanspec/changes/c465-…/design.md` + **c525**。

冻结期内允许：c460/c491 harness 回归、DESIGN 文档、包内通用缺口。

---

## 四、SSOT 指针

| 主题 | 路径 |
|---|---|
| 分层 / 导出 / 冻结 | 根 + `src/AGENTS.md`、`src/app/tui/AGENTS.md` |
| 产品架构图 | `docs/architecture/` |
| 包边界 / vs pi | `packages/xylitol-tui/AGENTS.md`、`PI_DELTAS.md` |
| 视觉 | `src/app/tui/DESIGN.md` + `design/` |
| How-to | `write-tui`、`test-tui-harness`、`write-surface` |
| 轨 P Prompt | `_prompts/track-p-tui-polish.md`（缺则用本文件 §二） |
| 轨 A 索引 | `_NOTE.md` |
