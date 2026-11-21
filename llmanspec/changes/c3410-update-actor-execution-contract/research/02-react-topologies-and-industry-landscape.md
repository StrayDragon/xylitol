# 调研底稿：ReAct 衍生拓扑谱系与行业横评

两路一手调研（2026-11-20），全论断带来源；抓取受限处标注未确认。

## 一、拓扑谱系与三门映射

| 拓扑 | 代表 | 长在哪 | 判定 |
|---|---|---|---|
| Plan-and-Execute | LangChain blog / LangGraph | workflow DAG 门（已有 <plan> 条目中间件） | 不动执行模型 |
| ReWOO | arXiv:2305.18323 | 拆两半：观察回灌可中间件；并行 worker 需并行 model call | 后半须动执行模型 |
| LLMCompiler | arXiv:2312.04511 | 并行 model call 门 | 须动执行模型 |
| Reflexion | arXiv:2303.11366 | 现有 decide_continue / 评审中间件同型 | 不需要新门 |
| supervisor / handoff（进程内） | OpenAI Agents SDK handoffs | handoff 预留门（r2921） | 小改 |
| supervisor（跨进程） | 同上 | 常驻实体与路由 | 须动执行模型 |
| durable workflow | Temporal docs | 跨 resume 执行器（点名 core seam 对象） | 完全体须动执行模型 |
| actor 框架本体 | AutoGen 0.4 / Cloudflare Agents | 常驻信箱 + 事件驱动 | 须动执行模型 |
| 后台 / 守护 agent | GitHub Copilot coding agent 等 | 触发器 + handoff 可起步 | 完全体动执行模型 |

结论：评审、Plan-and-Execute、进程内 handoff 三类从现有预留门长出；并行执行、durable workflow 完全体、actor 本体、多后台并发四类是本次重写的收益面。

## 二、行业八产品对照（进程模型 / 循环 / 扩展 / 会话）

- Claude Code：CLI 单进程（+SDK / 云）；单主循环 + 工具批；MCP + 五类 hooks + 插件；checkpoint 每轮快照可 rewind。
- Codex CLI：Rust workspace，app-server-daemon 常驻；core 单会话循环，thread→turn→item；rmcp-client + hooks/skills/plugin-host；rollout 文件 resume / fork。
- Gemini CLI：Node monorepo，acp 常驻模式；core 循环 + 远端 subagent 走 A2A；MCP + 自定义命令；自动存盘 + 影子 git checkpoint。
- grok-build：Rust，leader 常驻 + ACP server；runtime 内循环 + 并行子会话；MCP/skills/hooks + 插件市场（trust 双门禁）；全会话 jsonl 落盘。
- Aider：Python 单进程；architect→editor 两段式；无官方 MCP（未确认）；git 提交当状态边界。
- OpenHands：SDK 嵌入或 agent-server（REST+WS）；无状态事件驱动循环；MCP 与原生工具同构；append-only 事件日志即记忆（事件溯源）。
- Cline：VS Code 扩展进程内；Task 顺序循环，Plan/Act 双模式；McpHub + marketplace；按任务分文件 + 影子 git。
- Copilot coding agent：云端三分（client/session/environment）；循环闭源未确认；session 跨客户端接续。

## 三、差异化定位（xylitol 视角）

1. SDD 驱动开发为行业独点。
2. 端纯 client + 统一文本信封无第二家完全对齐（Codex daemon / grok-build leader 是外挂式）。
3. 官方插件一能力一包可整包替换未见同类。
4. 会话格式当公共契约属少数派（同队：grok-build、OpenHands）。
5. TUI-first 本地优先是主流，不构成差异。
6. 云端形态是分水岭：本仓本地 Host 可选常驻居中。

主要来源：各产品官方文档与仓库（code.claude.com、github.com/openai/codex、google-gemini/gemini-cli、xai-org/grok-build、aider.chat、docs.openhands.dev、docs.cline.bot、docs.github.com）；论文 arXiv 2305.18323 / 2312.04511 / 2303.11366 / 2511.03690。
