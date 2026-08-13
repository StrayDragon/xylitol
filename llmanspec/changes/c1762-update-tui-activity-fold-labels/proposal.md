---
depends_on:
- c1761-fix-tui-activity-fold
branch: sdd/c1762-update-tui-activity-fold-labels
base_sha: 50f988f7a974a0879e80e706c8d8c0883157003f
checkpointed: false
---

# Activity 折叠条文案：诚实词表与层级

> **一句话**：折叠条只写真实发生的事；Worked for / Edited / Explored / Ran 分层且 Title Case；禁止再把 thinking、compaction、MCP 写成 Explored。

> 前置已归档：[`c1761`](../archive/2026-08-13-c1761-fix-tui-activity-fold/)（信封套簇、live window）。本票 **改** 簇头语义与计数，**不**重做嵌套状态机、键位、YAML。Session 复刻：[`research/session-460ad16e-fold-honesty.md`](./research/session-460ad16e-fold-honesty.md)。

## Why

c1761 把中间操作收进信封/簇之后，簇头仍会**虚构**类目：Thinking/Ask/Compaction/未知工具在计数为空时被写成 `files = 1`，于是头行变成 `Explored 1 file`。session `460ad16e-874f-418f-8ca0-dabc58f89320` 的「cool 总结下」一轮只有 compaction（101494 tokens）+ Error，零 read，展开信封却出现 Explored——与 Cursor 的 Worked for 套真实 Explored/Edited、以及「不能虚构」冲突。att24 只要求英文计数模板、禁止假 +/- / 假时长，**没有**授权用 file 占位；实现违反了产品意图。需要重钉各层动词，并允许改 live spec。

## What Changes

1. **分层词表（Title Case）**，Worked / Explored 不是同一根条的两种文案：
   - 信封：`Worked for {duration}`（缺戳省略时长，禁止伪造）。
   - 簇：文件层 **Edited XOR Explored**；有 shell 才追加 `Ran N commands`。
   - 回退头：仅 thinking → 流式 `Thinking`，结束后 `Thought` / `Thought {dur}`；仅 MCP/未知 → `Used {tool}`；仅 Ask → `Asking questions`。
   - 仅 compaction：**不画簇头**，直接露出已有 `[compaction] Compacted from N`。
2. **计数诚实**：Edited/Explored 的 N 为去重 path；N=1 写 basename；N>1 写 `N files`。命令按调用次数。无 path 的 search 不加假文件数，但仍让该簇进入 Explored 侧。
3. **+/-**：簇头在 ToolEnd 增量聚合已有可靠 `display_diff` 的 `+N -M`；没有则省略。不阻塞 [`c1770`](../c1770-add-worktree-snapshot/proposal.md)（工作树整轮/逐步盘变更另票）。
4. **live 进行时**与过去式对齐：有改写 → `Editing`；否则读/搜 → `Exploring`；shell → `Running`；封口后改 `Edited` / `Explored` / `Ran`。
5. **展示**：继续扁平同列（c1761）；语义树不变。Error / ScrollNotice 仍不进信封。
6. **改 att24**（及 att33 进行时措辞若与本词表冲突）：以本票讨论为 SSOT，不保留 `files = 1` 占位。

## Capabilities

| capability | 角色 |
|---|---|
| `app-tui-transcript` | att24 簇头诚实词表；att33 进行时 Editing/Exploring/Running；att23 树仍信封套簇套块，仅 compaction 独簇不套第二根头 |
| `app-tui` design | `design/activity-fold.md` 文案 MUST 与词表同步 |

## 已拍板

| 项 | 决定 |
|---|---|
| 主轴 | 钉各层动词；可以改 spec |
| 簇头混合 | 文件层互斥：有 edit/write/apply_patch/str_replace → 只写 Edited；否则有 read/ls/search → Explored。禁止 `Edited …, explored …` |
| 命令 | 真实 shell 才写 `Ran N commands`（进行时 `Running`）。bash-only **不**叫 Explored |
| 辅助类 | thinking / MCP / compaction **不**进混合簇头；四类（文件 Edited/Explored + Ran）皆空才回退 |
| MCP / 未知 / todo_* | `Used`，永不进 Explored；**不**为 Todo 单开簇头类目 |
| 中途思考 | **不改 att34**：同一轮中途 thinking 留在打开簇；簇头跟真实工具走，L1 才是 `Thought {Ns}` |
| compaction 独簇 | 不另画簇头 |
| 计数 | 去重 path；N=1 basename；命令按次 |
| +/- | ToolEnd 聚合 `display_diff`；c1770 不进本票 |
| 大小写 | Edited / Explored / Ran / Running / Thinking / Thought / Used 与 Cursor 同类大写 |
| 树 / 缩进 | 信封 → 簇 → 块；扁平同列，不缩进 |
| L1 块动词 | 工具/compaction chrome **不改**；thinking 块统一 Title Case：流式 `Thinking`，结束后 `Thought` / `Thought {dur}` |
| live 占位 | **MUST NOT** 画 `Planning next moves`；busy 走状态条 |
| 簇切刀 att34 | **不改**：仍以助手正文封口 |

## 与邻接边界

| 对方 | 本票 | 对方保留 |
|---|---|---|
| c1761 | 改簇头/计数/进行时措辞；去掉 Planning 占位；Thinking→Thought 时长 | 嵌套状态、live window 结构、YAML、键位 |
| c1760 | 仍禁止假时长 / 假 +/- | 段时钟 |
| c1770 | 不依赖、不阻塞 | 工作树 snapshot 整轮差分；日后可接信封级 +/- |
| Compact 头行 | 不改 `[compaction] Compacted from N` 块文案 | 三角同行仍属既有 quick |

## Out of scope

- 打开 att34 改成「一轮一个聚合簇」
- L1 子项改成 Cursor 的 Read/Ran/Used/Waited 词（thinking 的 Thinking/Thought 除外）
- 用缩进表达嵌套
- c1770 snapshot / 逐步工作树 diff / bash 改盘行级 +/-
- Web 实现
- i18n

## Open Questions

| # | 题 | 决议 |
|---|---|---|
| 1 | 范围 vs 打开 att34 | **已钉**：不改簇切刀；先修词表 |
| 2 | 簇头动词集合 | **已钉**：Edited XOR Explored + 可选 Ran；空则 Thought/Used/Asking；compaction 独簇无簇头 |
| 3 | 工具归属 | **已钉**：改写→Edited；read/ls/search→Explored；bash→Ran；MCP/未知/`todo_*`→Used（不单开 Todo 簇头） |
| 4 | N 与文件名 | **已钉**：去重 path；N=1 basename |
| 5 | 树与 compaction | **已钉**：扁平；独簇不套头 |
| 6 | +/- | **已钉**：ToolEnd `display_diff`；c1770 另票 |
| 7 | 大小写 | **已钉**：Ran / Running 等同级大写 |

## Ethics

- risk_level: low
- prohibited_actions: 用 file 占位虚构 Explored；伪造 +/- 或时长；本票打 git 工作树当 diff 真值；默认分支改 live specs
- required_evidence: thinking-only 不见 Explored；compaction-only 不见 Explored 且无第二根簇头；MCP 为 Used；edit+read 只见 Edited；session `460ad16e`「cool 总结下」手测与研究笔记一致
- escalation_policy: 若要把 snapshot +/- 塞进本票 → 停下来拆票

## 测试边界（seam）

复用既有产品 harness / `activity_fold` 单测（与 c1761 同缝），**不**新开 CLI 子进程面：

- `count_cluster` / `format_cluster_body` / `format_cluster_header`（词表、XOR、去重、回退、禁止 files=1）
- scrollback paint：compaction 独簇无簇头；信封 L3 仍藏 compaction
- live tape：进行时 Exploring vs Editing vs Running
- `/debug activity-fold-live` / `activity-fold-resume`：夹具若仍断言假 Explored 则改夹具，不放宽诚实性

session `460ad16e` 为研究/手测，**不**进 `just qa`。

## Start readiness

| 项 | 值 |
|---|---|
| **ready_for_start** | **yes**（词表已锁；规划壳齐） |
| Specs | 绑定分支后改 att24 / att33（及 att23 仅 compaction 独簇句）；`feature: false` unit |
| 禁止 | 默认分支改 `llmanspec/specs/**`；把 c1770 / 非 thinking 的 L1 工具动词重命名算进本 change |
