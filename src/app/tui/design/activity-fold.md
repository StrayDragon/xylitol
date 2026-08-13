---
version: "alpha"
name: "activity-fold"
description: "Activity fold — nested envelope/cluster/block; streaming live queue (Planning next moves)."
tokens_from: "../DESIGN.md"
components:
  fold-header:
    textColor: "{colors.muted}"
  fold-live-tail:
    textColor: "{colors.muted}"
  diff-plus:
    textColor: "{colors.success}"
  diff-minus:
    textColor: "{colors.error}"
---

# Activity fold（嵌套收纳 + 流式队列）

> Token 根源：`{colors.*}` → [`../DESIGN.md`](../DESIGN.md)。
> 静图：[`playground/`](./playground/) 槽 `activity-fold`。
> 块级 thinking/tool 折叠仍见 [`expandable.md`](./expandable.md)；本文件管 **段/簇/流式队列**。
> 规划：[`c1761`](../../../llmanspec/changes/c1761-fix-tui-activity-fold/proposal.md)。

对齐 Cursor Agent 窗：低级操作自动收进上一聚合块；默认只留一个最小化入口；点击揭开已算好的 trace。

## MUST

### 折叠标记

1. 可折头行字形遵守 [`glyphs.md`](./glyphs.md) 与 att19：收起 `▸`、展开 `▾`（ascii `>` / `v`）；**行首**一列。本票 **MUST NOT** 改到行尾。
2. `Planning next moves` **MUST NOT** 画三角。

### 流式打开簇（队列）

打开簇分两截：**sealed**（已结束的低级操作）+ **inflight**（正在处理）。

3. **默认收起**：隐藏该打开簇的 sealed 子块与 `Edited…` 统计头。其上方已封簇（-3）仍可按其自身态显示（计数冻结，仍可点开看 L1）。底部 **仍画 inflight**：无可展示正文且无进行中操作时才是 `Planning next moves`；否则为 Thinking 体 / Ask 块 / `Editing tests.rs` 等。`Planning next moves` 整行可点、无三角。
4. **点标题展开**：整行可点（att22 三角列-only 的例外，同类 att30 hint 带）。揭开的是 **已经增量算好** 的 sealed 列表，**MUST NOT** 点击时全量重算。
5. **展开后形态**（自上而下）：
   1. 统计摘要头 `Editing N files, …` + 可选 `{colors.success}` `+N` / `{colors.error}` `-N`（无可靠 diff 则省略 +/-）
   2. 摘要头行首 **折叠三角 `▾`** — 这是 **唯一** 收起入口
   3. sealed 细账（read / grep / thinking / …）**MUST** 仍走 [`expandable.md`](./expandable.md) 块级独立折叠（点块头三角，不是点簇头）。playground 静图可以把这些 L1 画成不可点标签。
   4. 原标题 `Planning next moves`（或当前 inflight：`Thinking` / `Asking questions` / `Editing tests.rs`）留在 **最底**，作为本段流程终点标记
6. **点摘要头三角收起**：回到第 3 条紧凑态（隐藏 sealed 与统计头，**仍留 inflight**）。
7. 三角何时画在摘要头：仅当 sealed **非空**。尚无 sealed 时只有底部标题，无 `Edited…` 行。
8. 类目计数在 **ToolStart** 写入摘要缓存；`+/-` 仅 **ToolEnd** 且有可靠 diff。
9. Ask `Waiting`：底部标题为 `Asking questions`；Ask 块 **MUST** 可交互，不得折没。
10. 助手正文 **第一个非空白字符** 封口本簇（进行时 `Editing` → 过去式 `Edited`），簇进入 -3 冻结。

### 旧 turn 信封

11. `stream_collapse: envelope`（默认）：已结束 turn（resume/rebuild 为全部已结束轮；turn-end 为超窗）折叠为 `Worked for {duration}`（缺戳则省略时长，禁止伪造）。流式当前 turn MUST NOT 套信封。
12. 信封折叠可见：User + `Worked for` + 该 turn **最后一段** Assistant。中间正文与 Todo/Compaction/工具一并收纳。ScrollNotice / Error **MUST NOT** 进信封。
13. 信封展开：头行 `▾ Worked for` **仍在**（可再折）；其下见簇头（可再点开）。展示与未折叠细账 **同一列**，MUST NOT 用缩进表达信封/簇/块层级。live window / 展开态 **仍画** 夹心助手正文（与折叠信封的 B 不同）。

### 性能与动画

14. 摘要字符串与 sealed 下标在 ToolStart/End **增量**更新。展开只读缓存 + 既有块 paint。
15. 流式 invalidate 仅 `-1` 与打开簇头（及展开中的 sealed 行）。-3 **MUST NOT** 因打开簇 toggle 失效。ath25：禁止全历史 Assistant Markdown 重解析。
16. **TUI 高度变化 = 一次 paint 揭开/收起**（差分渲染）。**MUST NOT** 要求逐行滑入动画。playground 槽可用芯片跳预设，且 **transcript 行可点** 切换折叠；这是静图示意，不是产品运行时。

### 状态条

17. 输入框上方 busy 短词仍为 `Working` / `Running {name}`（[`status.md`](./status.md)）。`Planning next moves` **MUST NOT** 进状态条。
