# app/tui ↔ pi coding-agent 刻意差异台账

> **目的**：对照 `../pi/packages/coding-agent` interactive 做 UX 对齐时，**不得静默覆盖**本文件列出的 xylitol 产品决议。
> **定位**：本面是 xylitol 产品 TUI（host + XyDriver seam），不是 pi interactive 的 1:1 port。包层差异见 [`packages/xylitol-tui/PI_DELTAS.md`](../../../packages/xylitol-tui/PI_DELTAS.md)。
> **不是**进度板；能力差距清单见 `design/session-tree-vs-pi.md`。稳定边界见本目录 `AGENTS.md`。
> 新增刻意差异时：**先改代码与测试（或明确不实现），再在本表加一行**；回退差异须显式评审。

对齐源路径（历史参考）：`../pi/packages/coding-agent`（interactive / tree / travel）。

会话 slash 迁移调研工件：`llmanspec/changes/c1005-*/design.md`、`c1010-*/design.md`、`c1015-*/design.md`；计划报告修正见 `../pi/_PLAN_REPORT.md`「调研修正」节。

---

## 如何使用

1. 从 pi 拉行为或补丁前，先扫本表「不得回退」列。
2. 若 pi 变更触及某行主题，默认 **保留 xylitol 侧**；只有产品明确要求才改决议并更新本表。
3. 纯 bugfix（两边语义一致）可对齐 pi，不必记入本表。
4. 包组件行为差异记在 `packages/xylitol-tui/PI_DELTAS.md`，不要重复抄进本表。
5. 缺能力优先按 xylitol 产品需求设计；需要扩展点时先留 hook，再接具体策略。

---

## 刻意差异（normative）

| ID | 主题 | pi coding-agent | xylitol `src/app/tui` | 不得回退 |
|---|---|---|---|---|
| A01 | Travel 时分支摘要 | travel / 切分支时可走 LLM（或同类）生成 branch summary 写回树 | **不做** travel 时自动摘要；树节点文案来自 entry / label / 既有 summary 字段。若以后要策略，经 **hook / 扩展点** 注入，不内置默认 LLM 路径 | 是 |
| A02 | 同会话 fork 形态 | 树内 `/fork` 等可在同会话 MessageHistory 上开兄弟枝（再配合新会话文件语义）；slash `/fork` 开 **user 消息选择器** | 产品 **Shift+F / `/session-fork`** = `XyDriver::fork_session`（**新 child session** + switch）；同会话兄弟枝靠 **travel 改 leaf 后再发消息**（`parent_id`←当前 leaf）长出来。demo `agent_demo` 的同会话 Shift+F 是原型，**不是**产品默认语义。**不开** pi 式 user 选择器 | 是 |
| A03 | Slash 命名 | 短名：`/tree` `/fork` `/export` `/import` `/compact` `/resume` `/quit` … | 选中迁移命令用 **`session-*` 前缀**（如 `/session-tree`）；**旧名无效**（unknown）。`/model` `/exit` 仍短名；`/exit` 仍认 `quit` | 是 |
| A04 | `/session` 形态 | 无参 → scrollback **info/stats 转储**（非操作菜单） | 对齐 dump（c1015）；**不做**「SessionOperations 覆盖层 / 子命令板」 | 是 |
| A05 | Compact 自定义指令 | `/compact <instructions>` 可传自定义压缩提示 | `Command::Compact` 无 instructions 字段 → **仅无参** `/session-compact`；带参 usage 错误 | 是 |
| A06 | Import 确认 UI | extension confirm 对话框 | editor 槽 **Yes/No SelectList**（不解冻 Trust Choice stub） | 是 |
| A07 | Clone vs fork | `/clone` = leaf `fork(at)`；`/fork` = user 选择器 | `/session-clone` = leaf **恒 At** + switch；`/session-fork` 仍遵守 A02（user→Before / 非 user→At）。二者 MUST NOT 混用语义 | 是 |
| A08 | Resume scope=All | 多 project 根目录 `listAll` 全局列举 | 单 `sessions_dir` 下全部 jsonl；scope=Current 按 header `cwd` 过滤 | 是 |
| A09 | tool/diff 块键 id | 无独立 Alt+E app id（或不同命名） | **`app.tools.blocks`** = Alt+E（产品特有）；`app.tools.expand` = Ctrl+O 视口 | 是 |
| A10 | Skill 调用呈现 | `/skill:name` → `<skill>…</skill>`；scrollback **每条** skill 用 `SkillInvocationMessage` 色块折叠/展开 **SKILL.md** | 产品用 **内联多 `$name`**（非 `/skill:`）。提交时 **读 SKILL.md 注入模型上下文**（静默，可多引用）。Scrollback：**只在用户消息内**用特殊色（如紫）高亮 `$name`；**MUST NOT** 另加系统消息行、N 个 skill 色块、footer `skills:N`、**`/session` / `/status skills` skill 清单**。验收以 **注入/read 断言**为准，不以 TUI 元素为主门禁。**正交**：启动/`/reload` 的 loaded-resources 槽（c1135）是**目录可见性**（skills/MCP 摘要），不是调用刷屏 | 是 |
| A11 | Skill 发现路径 | 多源：`~/.pi/agent/skills`、`~/.agents/skills`、项目 `.pi`/`.agents`（祖先）、packages、settings、CLI | **产品路径**：`~/.xylitol/skills`、`~/.agents/skills`、`{cwd}/.xylitol/skills`、`{cwd}/.agents/skills`（Trust 闸项目侧）。同名优先级 **`.xylitol` > `.agents`**，且 **project > user**。**对齐** agentskills 元数据：`disable-model-invocation`、name 校验警告、system `<available_skills>` + read-tool 引导文。**不做**全量 pi 祖先递归 / packages / ignore 文件 | 是 |

### 对齐（非差异，备忘）

| 主题 | 双方行为 |
|---|---|
| Export 默认格式 | **默认 HTML**；路径以 `.jsonl` 结尾才 JSONL |
| Resume 入口 | 无参开会话列表；选中 switch；预览按终端比例软顶；**默认隐藏** session id，**Ctrl+U** 展开完整 id（c1530） |
| Tree 入口 | slash / 快捷键开 MessageHistory 树（xylitol 另保留双 Esc） |
| Skills catalog → system | Trust 后发现；`<available_skills>` XML；reload 不改历史（c1085） |

---

## 分叉可见性（手测备忘）

| 现象 | 原因 | 怎么看到分叉 |
|---|---|---|
| `/debug session-tree-multiturn` / `labeled` 打开是一条脊 | 线性夹具 | 用 **`/debug session-tree-branched`**，或见下「同会话兄弟枝」手测 |
| 一直 Fake 聊天、从不 travel | 每条消息挂在 tip leaf → 永远一条链 | 先 travel，再发 |
| Shift+F / `/session-fork` 后树仍像一条链 | 你已切到 **新 session**；父会话树不会自动出现「旁路子会话」节点 | 在父会话里 travel+续聊看兄弟；或分别打开父子 session 对比 |

**同会话兄弟枝（产品已有机制）**

1. 有多轮历史（手聊或 `/debug session-tree-multiturn`），**或** 直接 `/debug session-tree-branched`。
2. 若未用 branched：双 Esc / `/session-tree` → 选中**中部**节点 → Enter travel → 再发一条用户消息。
3. 开树：应能看到同一父节点下 **≥2 个孩子**（原枝 + 新枝）。fold / ←→ 分支跳转即可验。

**跨会话 fork（产品已有机制）**

- 树内 Shift+F，或叶上 `/session-fork` → `fork_session` + `switch_session`（user→`Before` / 非 user→`At`）。

`session-tree-branched` 预置兄弟枝；PTY：`pty_product_fake_session_tree_branched`。线性夹具测不出分叉 UI，不等于没有机制。

---

## 变更记录（短）

| 日期 | 变更 |
|---|---|
| 2026-07-14 | 建表；A01 明确不做 travel 分支摘要（撤 c695）；A02 产品 fork=新 session vs 同会话 travel 分枝 |
| 2026-07-14 | `/debug session-tree-branched` 预置兄弟枝；PTY `pty_product_fake_session_tree_branched`（raw 断言，避 CapturedScreen 长 scrollback 不同步） |
| 2026-07-15 | 会话 slash 迁移调研：A02 钉 `/session-fork`（非 user 选择器）；增 A03–A06；手测备忘 `/tree`/`/fork`→新名；对照 `../pi/_PLAN_REPORT.md` |
| 2026-07-15 | c1020：`/session-new` `/session-clone` `/session-name`；A07 clone(At) ≠ session-fork |
| 2026-07-15 | c1065：Resume 面板 P0–P2；A08 单 sessions_dir All ≠ pi 多根 listAll |
| 2026-07-23 | c1530：Resume 预览按终端比例软顶；**Ctrl+U** 切换完整 session id（默认隐藏） |
| 2026-07-16 | c1090：app.* 目录 + 热重载；A09 `app.tools.blocks` |
| 2026-07-16 | A10：多 `$skill` → 用户消息内紫色高亮 + 静默注入 SKILL.md；废弃 /session·/status skills 观测面；验收验注入不验 TUI |
| 2026-07-16 | A11：skills 发现路径子集 vs pi 多源；对齐 disable-model-invocation / 碰撞 / available_skills 引导文 |
| 2026-07-16 | A11：默认发现 `.agents/skills`（user+project）；优先级 `.xylitol` > `.agents`，project > user |
| 2026-07-16 | A10 澄清：c1135 loaded-resources = 目录可见性，≠ 调用刷屏 / 不替代注入验收 |
