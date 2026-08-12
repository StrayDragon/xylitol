# Pi / Cursor：sub-agent 委托任务 UI 一手对照

- **日期**：2026-08-12
- **Change id**：`c2080-add-append-only-subagent-tui`
- **性质**：Change 调研（一手摘录与对照）；**不是** live specs；不改 `proposal.md` / `llmanspec/specs/**`

调研问题：Pi 与 Cursor 如何呈现子 agent；哪些事实支持 c2080「append-only / 无折叠 / 极简键 / spill」，哪些提示保留摘要折叠或独立面板。

---

## 1. Pi 事实

### 1.1 产品定位与进程模型

- 核心 **故意不内建** sub-agents；由 extension/package 实现。（`/home/l8ng/Projects/__straydragon__/pi/packages/coding-agent/docs/usage.md` L299–303；examples 索引 `docs/extensions.md` L2974）
- 官方示例：`packages/coding-agent/examples/extensions/subagent/`。每个委托 **另起独立 `pi` 子进程**，`--mode json -p --no-session`，隔离 context。（`README.md` L7–8；`index.ts` L3–5、L300、L344–350）
- 子进程 stdin 忽略；stdout 按行 JSON；stderr 收集。`AbortSignal` → `SIGTERM`，5s 后 `SIGKILL`。（`index.ts` L346–350、L410–419）
- 系统 prompt 写入 `os.tmpdir()` 下 `pi-subagent-*` 临时文件，结束后删除——**不是** session 旁 spill，而是短命 prompt 投递。（`index.ts` L239–246、L426–438）

### 1.2 流式与并行

- `execute(..., signal, onUpdate)`：解析 `message_end` / `tool_result_end` 后 `onUpdate({ content, details })`，父 TUI 工具行可流式刷新。（`index.ts` L324–330、L362–387）
- 模式：single / parallel（最多 8、并发 4）/ chain（`{previous}`）。（`README.md` L91–97；`index.ts` L33–34、L599–609）
- Parallel 运行中：`content` 为 `Parallel: done/total done, N running...`；各任务 `exitCode === -1` 表示仍在跑。（`index.ts` L627–637、L941–952）

### 1.3 折叠 / 展开与最终形态

- 呈现落在父会话 **工具结果槽** `renderCall` / `renderResult`（pi-tui `Text`/`Container`/`Markdown`），**不是**独立 alt-screen 子会话面。（`index.ts` L718–760、L762+；扩展约定 `docs/extensions.md` L2263–2324）
- **默认 collapsed**：状态图标 + agent 名；display items 截断（single 末 10 条；chain/parallel 每任务末 5）；文本项非展开时最多 3 行；提示 `(Ctrl+O to expand)`。（`README.md` L101–104、L174；`index.ts` L35、L771–784、L832–842、L921–937、L1006–1026）
- **expanded**（`{ expanded }`）：完整 task、全部 tool call 格式化行、最终 assistant text 用 `Markdown`、usage；parallel 仅在 **不在运行** 时走完整 expanded 容器。（`README.md` L106–110；`index.ts` L794–829、L862–918、L954–1003）
- 父模型可见结果：single/chain 取最终 assistant text；parallel 每任务输出 **50 KB** 截断，全文仍在 `details`。（`README.md` L116–117、L175；`index.ts` L36、L193–201、L664–679）

### 1.4 键位 / 中止

- 扩展文档：工具详情应用 `expanded`；展开键 id 为 `app.tools.expand`，默认 **`ctrl+o`**。（`docs/keybindings.md` L159；`docs/extensions.md` L2288–2322）
- Interactive：`app.interrupt` 默认 **`escape`** = Cancel/abort；`app.clear` 默认 **`ctrl+c`** = 清编辑器/二次退出——与 README「Ctrl+C propagates」措辞不完全同名，但 abort 经 `signal` 杀子进程是代码事实。（`docs/keybindings.md` L123–124；`README.md` L12、L169；`index.ts` L410–424）
- Subagent 未单独注册专用键；复用全局「工具输出展开」。

---

## 2. Cursor 事实（公开一手）

下列仅来自 `cursor.com` / `docs.cursor.com` changelog 与 help；**未找到**描述「子 agent 流折叠三角 / 块级 Ctrl+O / 键盘折叠」的官方 UI 细则。

### 2.1 委托与可见性语义

- Subagents：独立 context；父收到 **最终结果/摘要**；中间输出留在子侧，避免挤爆主对话。（https://cursor.com/docs/subagents — Context isolation；Built-in 段「parent only sees the final summary」）
- Foreground 阻塞至完成；Background 立即返回、独立跑；`is_background` 配置字段。（同页 Foreground vs background；Configuration fields）
- 并行：父在一条消息里发多个 Task tool call。（同页 Parallel execution）
- FAQ：**Background subagents 把输出写到 `~/.cursor/subagents/`**，父可读文件看进度——官方 spill/旁路路径。（同页 FAQ「How do I see what a subagent is doing?」）
- 失败：子返回 error status；父可 retry/resume。（同页 FAQ）
- 可用面：editor、CLI、Cloud Agents。（同页开篇）

### 2.2 并行 / 多面板 / 流式（changelog + Agents Window）

- 2.4：并行、自有 context、可配 prompt/tools/models；默认 explore/terminal/parallel 类；editor 与 **CLI** 均改善。（https://cursor.com/changelog/2-4）
- 2.5：async（父不阻塞）；可嵌套 spawn（有深度限制，见 docs FAQ）；「**better streaming feedback**」与更响应的并行；**Stopping the parent agent will always stop the child subagents**。（https://cursor.com/changelog/2-5）
- `/multitask`、Plan 上 **Build in Parallel** → async subagents。（https://cursor.com/help/ai-features/multi-agent；changelog 04-24-26 / 05-07-26）
- **Agents Window**：多 agent 工作区（sidebar 管理、pin）；云/本地切换；cloud subagents（`/in-cloud`、`/babysit`）。打开：Cmd+Shift+P → Open Agents Window。（https://cursor.com/docs/agent/agents-window；help multi-agent）

### 2.3 键盘（官方有，但非 subagent 折叠专属）

- 通用：Cmd+I / Cmd+L 侧栏；Cmd+E Agent layout；Agents Window 用命令面板打开；文件 Cmd+P / Cmd+Shift+F。（https://cursor.com/docs/reference/keyboard-shortcuts；agents-window 页）
- **未找到**官方一手规定「子 agent 卡片折叠/展开专用键」或「transcript 内 activity-fold」行为说明 → 记为缺口，不作推测。

---

## 3. 对照表

| 维度 | Pi（extension 示例） | Cursor（公开 docs/changelog） |
|---|---|---|
| 进程/隔离 | 独立 `pi` 子进程 + JSON mode | 独立 context window；云子还可独立 VM/branch |
| 父可见终态 | 最终 text；parallel 50KB/任务封顶；全文在 `details` | 父见最终 summary；中间不进主 context |
| 运行中可见 | 同工具行流式；collapsed 末 N 条 + 状态 | 「streaming feedback」有文字；背景进度可走 `~/.cursor/subagents/` |
| 折叠 | **默认折叠** + `app.tools.expand`（Ctrl+O） | 官方未描述折叠 UI；另有 Agents Window 总览 |
| 中止 | AbortSignal → kill 子进程；interrupt=Escape | 停父必停子（2.5 changelog） |
| 多任务 UI | 同一工具结果内并列任务块 | Task 并行 + Agents Window sidebar / 云移交 |
| Spill | prompt 用系统 tmp；完整轨迹在 tool details | 显式目录 `~/.cursor/subagents/` |

---

## 4. 对 c2080 的启示（事实 → 选项，不拍板）

### 支持「append-only / 无折叠 / 极简键 / spill」的事实

1. **Cursor 主对话减噪**：父默认只收最终摘要；噪声轨迹隔离——对齐 c2080「主会话不被细节淹没 / 旁路观察」与 roadmap「合并默认走摘要」。（docs/subagents；`docs/roadmaps/Sub-Agent编排.md` 回收压缩支线）
2. **Cursor 文件 spill**：`~/.cursor/subagents/` 作进度/输出旁路——与草案「session 旁唯一临时目录 + transcript 关联」同构可对照（路径策略不同，**契约形态**相近）。
3. **Pi 对模型侧的硬帽**：parallel 50KB/任务 + 「Full output preserved in tool details」——支持「可见面矮上限 + 完整内容另存」而非无限展开主屏。
4. **Pi 无独立子 TUI 面**：子输出挂在父工具行——支持「第二条瘦面 / 旁路」与「不在主 AO chrome 堆完整折叠故事」可拆选型（c2080 Open Questions：嵌套 vs 独立入口）。
5. **极简键**：Pi 子 agent 无专属键位表，只借全局 expand/interrupt——支持「瘦面几乎无产品快捷键、abort 进最小闭集」。

### 提示「保留摘要折叠或独立面板」的事实

1. **Pi 默认 UX 就是折叠摘要 + 按需展开**：流式时 collapsed 仍显示末几条 tool/text；并行跑完前 expanded 完整 Markdown 容器还不开放——说明「一味 append 全量」并非 Pi 选的默认减噪手段。
2. **Pi Ctrl+O / `expanded` 是工具层一等公民**（extensions.md best practice「Support expanded」「Keep default view compact」）——若 xylitol 主线已有 fold 族，子任务「摘要条 + 偶发展开」与现有学习成本同构，而非对立。
3. **Cursor Agents Window / sidebar**：多 agent 用**独立管理面**而非塞进单一 transcript——支持「独立面板 / 第二入口」选项，而非只做主屏 AO 内无折叠长流。
4. **Cursor streaming feedback（changelog 陈述）+ 可读 spill 文件**：暗示用户侧仍要**某种进行中可见性**；若瘦面完全无摘要折叠，需另有状态条/spill 打开路径，否则「可看见」仅落在父模型摘要。

### 与草案 Open Questions 的映射（仅指向，不裁决）

| 草案问题 | 一手暗示 |
|---|---|
| 载体 Inline vs AO 小窗 | Pi=父工具行内嵌；Cursor=主对话摘要 + 可选 Agents Window——两种都成立 |
| 嵌套 vs 独立 TTY | Pi 同 TTY 工具卡；Cursor 可云/Agents Window 分面 |
| 最小键位 | Pi：interrupt +（可选）expand；Cursor 公开文档无子卡折叠键 |
| 面 vs 编排另票 | Pi subagent 是 extension 工具；Cursor Task/内建 explore——编排与呈现可分层交付 |

---

## 5. 未覆盖缺口

- Cursor：**未找到**官方一手描述 editor/CLI 内 Task/subagent **卡片折叠、键位、行级 UI**；仅有语义（摘要/并行/async）、spill 路径、Agents Window、changelog「streaming feedback」。
- Pi：子进程 JSON 事件与父 interactive 工具组件接线细节未全文追踪（已确认 `renderResult`/`toolOutputExpanded` 路径存在）。
- 未对照 xylitol 现有 `full_output_path` 实现代码（属本仓实现，非本次外部一手范围）。
- 社区论坛/二手测评未用作论断依据。
