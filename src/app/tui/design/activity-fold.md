---
version: "alpha"
name: "activity-fold"
description: "Activity fold — nested envelope/cluster/block; streaming Thinking → Thought; Ask waiting Asking questions."
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
> 规划：[`c1762`](../../../llmanspec/changes/c1762-update-tui-activity-fold-labels/proposal.md) 词表；嵌套结构仍见已归档 c1761。

对齐 Cursor Agent 窗：低级操作自动收进上一聚合块；默认只留一个最小化入口；点击揭开已算好的 trace。

## MUST

### 折叠标记

1. 可折头行字形遵守 [`glyphs.md`](./glyphs.md) 与 att19：收起 `▸`、展开 `▾`（ascii `>` / `v`）；**行首**一列。本票 **MUST NOT** 改到行尾。
2. `Asking questions`（Ask 等待的 live 尾行）**MUST NOT** 画三角；整行可点展开打开簇。**MUST NOT** 画 `Planning next moves`。

### 流式打开簇（队列）

打开簇分两截：**sealed**（已结束的低级操作）+ **inflight**（正在处理）。

3. **默认**：打开簇有工具时 **MUST** 画带三角的簇头（进行时 `Editing` / `Exploring` / `Running`）。流式工具块是该簇子项，**默认折叠**（不弹出 Write/Read/MCP 正文）。点簇头三角才展开子项（含流式）。同一打开簇内 ToolEnd / 多次调用归并 **MUST NOT** 自动收起已展开的子项（避免屏幕跳动）。paint **MUST NOT** 每帧改折叠态。助手正文封口后该簇冻结为过去式。其上方已封簇（-3）仍可按其自身态显示。无 thinking/Ask/工具时 **MUST NOT** 画假占位尾行（busy 短词走状态条）。Thinking 流 **MUST** 画一条 `Thinking` 簇头（可点展开，默认折叠正文，无时长、无 `(Ctrl+T)`）；流结束后 **MUST** 更新为 `Thought`，有可靠起止墙钟 **MUST** 写 `Thought {Ns}`（如 `Thought 17s`）；resume 无戳则省略时长；**MUST NOT** 流式每帧刷新时长。**MUST NOT** 同时画 `Thinking`/`Thought` 簇头与 thinking L1 `(Ctrl+T)` 头。Ask 块仍走各自动态块，不是无三角的 `Editing` 尾行。
4. **点簇头三角展开**：揭开的是 **已经增量算好** 的 sealed 列表，**MUST NOT** 点击时全量重算。Ask 等待时点 `Asking questions` 整行同样展开打开簇（att22 三角列-only 的例外，同类 att30 hint 带）。
5. **展开后形态**（自上而下）：
   1. 统计摘要头：文件层互斥 `Editing {name|N files}` 或 `Exploring {name|N files}`，有 shell 才追加 `Running N commands`，可选 `{colors.success}` `+N` / `{colors.error}` `-N`（无可靠 diff 则省略 +/-）。MUST NOT 并列 explored，MUST NOT 把纯 read 写成 Editing。仅 thinking：流式为 `Thinking`，结束后为 `Thought` / `Thought {Ns}`。
   2. 摘要头行首 **折叠三角** — 展开/收起该簇子项的入口
   3. 子项细账（read / write / grep / thinking / …，**含流式**）**MUST** 仍走 [`expandable.md`](./expandable.md) 块级独立折叠（点块头三角，不是点簇头）。playground 静图可以把这些 L1 画成不可点标签。
   4. Ask 等待时 `Asking questions` 留在 **最底**；否则无 Planning 占位行
6. **点摘要头三角收起**：隐藏该簇子项（含流式工具块），**留下带三角的簇头**；Ask 等待时仍见底部 Asking questions。
7. 三角何时画在簇头：该簇有可计活动（工具/thinking/Ask 回退头）即画。尚无工具、只有 busy 时无簇头。仅 Compaction 的簇不画第二根簇头（att23）。MUST NOT 用 `...` 当文件名。助手正文（夹心或该轮最后一段）**MUST NOT** 被收进簇。
8. 类目计数在 **ToolStart** 写入摘要缓存；`+/-` 仅 **ToolEnd** 且有可靠 diff。
9. Ask `Waiting`：底部标题为 `Asking questions`；Ask 块 **MUST** 可交互，不得折没。
10. 助手正文 **第一个非空白字符** 封口本簇（进行时 `Editing` / `Exploring` / `Running` → 过去式 `Edited` / `Explored` / `Ran`），簇进入 -3 冻结。仅 thinking 的簇头流式为 `Thinking`、结束后为 `Thought` / `Thought {Ns}`；仅 MCP/未知为 `Used`；仅 Compaction **MUST NOT** 再画簇头。MUST NOT 用文件占位虚构 Explored。

### 旧 turn 信封

11. `stream_collapse: envelope`（默认）：已结束 turn（resume/rebuild 为全部已结束轮；turn-end 为超窗）折叠为 `Worked for {duration}`（缺戳则省略时长，禁止伪造）。流式当前 turn MUST NOT 套信封。
12. 信封折叠可见：User + `Worked for` + 该 turn **最后一段** Assistant。中间正文与 Todo/Compaction/工具一并收纳。ScrollNotice / Error **MUST NOT** 进信封。
13. 信封展开：头行 `▾ Worked for` **仍在**（可再折）；其下见簇头（可再点开）。展示与未折叠细账 **同一列**，MUST NOT 用缩进表达信封/簇/块层级。live window / 展开态 **仍画** 夹心助手正文（与折叠信封的 B 不同）。

### 性能与动画

14. 摘要字符串与 sealed 下标在 ToolStart/End **增量**更新。展开只读缓存 + 既有块 paint。
15. 流式 invalidate 仅 `-1` 与打开簇头（及展开中的 sealed 行）。-3 **MUST NOT** 因打开簇 toggle 失效。ath25：禁止全历史 Assistant Markdown 重解析。
16. **TUI 高度变化 = 一次 paint 揭开/收起**（差分渲染）。**MUST NOT** 要求逐行滑入动画。playground 槽可用芯片跳预设，且 **transcript 行可点** 切换折叠；这是静图示意，不是产品运行时。

### 状态条

17. 输入框上方 busy 短词仍为 `Working` / `Running {name}`（[`status.md`](./status.md)）。**MUST NOT** 画 `Planning next moves`（状态条也不进）。
