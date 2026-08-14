# Design: Activity 折叠词表

对照：[`proposal.md`](./proposal.md)；视觉 MUST 仍在 [`src/app/tui/design/activity-fold.md`](../../../src/app/tui/design/activity-fold.md)（apply 时改文案，不改嵌套交互）。复刻证据：[`research/session-460ad16e-fold-honesty.md`](./research/session-460ad16e-fold-honesty.md)。

c1761 的信封套簇套块、att34 助手正文切簇、扁平同列、live window 结构 **保持**。本票只改**头行怎么说话**，以及 compaction 独簇不要第二根条。

## 产品树

```text
[User]                                      始终外显
▸ Worked for {duration}                     信封：一轮一条；展开后头行仍在
   ├─ ▸ Edited {name|N files}[, Ran M]      簇：文件互斥 + 可选 Ran
   │    └─ L1 thinking / tool / ask
   ├─ ▸ Explored {name|N files}[, Ran M]
   ├─ ▸ Ran M commands                      仅 shell、无文件活动
   ├─ ▸ Thought / Thought {dur}             仅 thinking
   ├─ ▸ Used {tool}                         仅 MCP / 未知工具
   ├─ [compaction] Compacted from N         仅 compaction：不套簇头
   └─ （中间助手正文：信封展开才画）
[该轮最后一段 Assistant]
Error / ScrollNotice                        永不进树
```

折叠信封：User + `Worked for` + 最后 Assistant。
Worked for 与 Explored **不是**同一层的两种前缀；Explored 只在信封展开后作为簇头出现。

## 簇头算法

1. 扫描簇内 foldable middles。
2. 文件 path 去重：改写工具（edit/write/apply_patch/str_replace）→ Edited 集合；read/ls/search（grep/rg/glob/find 及名字含 search/grep）→ Explored 集合。
3. 文件层互斥：Edited 非空 → 只输出 Edited（升格，不附 explored）。否则 Explored 非空（含「有 search 但无 path」）→ Explored。
4. N=1 且有 basename → 写 basename；N>1 → `N files`；Explored 因无 path 的 search 成立且 N=0 → 只写 `Explored`（不写 `1 file`）。
5. shell（bash/shell/run_terminal_cmd/execute）调用次数 M>0 → 追加 `, Ran M command(s)`；无文件活动时整行就是 `Ran M commands`。
6. 若 3–5 皆空：
   - 只有 thinking → `Thought`（有可靠双端戳才加时长）
   - 只有 MCP/`mcp:*`/未知工具 → `Used {短名}`
   - 只有 Ask → `Asking questions`
   - 只有 Compaction → **不 emit 簇头**；paint 走既有 compaction 块
7. 可靠 `display_diff` 的 +/- 在 ToolEnd 累加到该簇头；无则省略。
8. 禁止：空类目 `files = 1`；未知工具当 file；compaction 当 Explored。

进行时（打开簇未封口）：Edited→`Editing`，Explored→`Exploring`，Ran→`Running`。禁止再把纯 read 写成 `Editing`。打开簇有工具时画带三角的簇头；流式工具是子项（**默认折叠**，点三角才见正文），不是无三角的 live 尾行。同一打开簇内 ToolEnd **不得**自动收起已展开的子项。Thinking 流与已完成 thinking 合并为一条 `Thought`，不得并排 `thinking` L1 头。助手正文始终外显、不进簇。

## Compaction 独簇 paint

信封 L3：compaction 仍随 middles 被收进 Worked for（c1761）。
信封展开：该簇若 middles **全部**是 Compaction → 不登记簇头 hit、不画 Edited/Explored/Compacted 第二行；按 L1 画 `[compaction]` 块（既有 Alt+E）。
thinking+compaction 而无文件/命令：走 `Thought` 头，compaction 当展开子项（不是独簇）。

## +/- 与 c1770

本票真值 = 工具结果里已有的 unified `display_diff`（edit/write）。增量点 = ToolEnd（与 att24 一致），不是每帧全量重扫，也不是 git status。

[`c1770`](../c1770-add-worktree-snapshot/proposal.md) 是 TurnStart 基线 → TurnEnd 整轮工作树切面，能覆盖 bash/sed 改盘；MVP 无产品入口、也不是逐步工具 snapshot。信封级「整轮 +/-」或逐步盘变更 **预留、不在本票接线**。禁止用 0 填充假装有统计。

## 实现落点（非 spec）

计数与格式在既有 summary 路径改；compaction 独簇在 scrollback 画簇头前短路。夹具 `/debug activity-fold-*` 若断言假 Explored 则改期望。
