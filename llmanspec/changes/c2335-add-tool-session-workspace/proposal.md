---
depends_on: []
branch: sdd/c2335-add-tool-session-workspace
base_sha: 637fc6eb3888bbae78c21d169b65cb056d9be41c
checkpointed: false
---

# 模型侧工具执行面穿会话工作区

attach 多工作区下，模型（LLM）调用的工具仍继承 Host 进程 cwd：TUI 在目录 A 启动、Host 在目录 B 监听时，模型侧 `bash` 在 B 起子进程，文件类工具的相对路径在 B 解析——与 bang `!`（已修，`BashExecOpts.cwd`）和会话 header / prompt env（已用会话工作区）分岔。本变更把 **会话工作区** 穿到工具执行上下文，使模型侧工具与人侧 bang、会话语境同源。

## Why

- A7 P0 已把 TUI 启动目录随 unary 交给 Host 并落进 agent cwd；但该 cwd 只用于 session header、prompt env 与 bang 执行路径。ReAct 工具批构造 `XyToolCtx` 时只携带 cancel + output uplink，工具无从得知工作区。
- 后果（TODO §2 A7 后续）：多工作区 attach 下 LLM 相对路径读写 / bash 命令落错目录，且错得静默——工具返回成功，文件却写进了 Host 进程目录。
- 验收对齐 `_TUI_MIGRATED_TODO.md` §0 验收 5：「TUI 在目录 A 启动、Host 在目录 B：`!pwd` / **工具** / 新会话 cwd = A」。bang 与新会话 cwd 已达标；本刀补齐「工具」。

## What Changes

- **协议缝**：`XyToolCtx` MUST 携带本次执行的工作区基目录；运行时（ReAct 工具批）从 run 冻结配置的 cwd 构造该字段（与 B3「run 开始冻结」同构：整段 run 含工具环用同一工作区）。未设置时回落 Host 进程 cwd（print/库嵌入同进程默认行为不变）。
- **bash 工具**：流式与非流式两条路径 MUST 都以 ctx 工作区为 shell 子进程 cwd。
- **文件类内置工具**（read/write/edit/patch/ls/grep/find）：相对路径 MUST 相对 ctx 工作区解析；绝对路径行为不变。
- **不改**：MCP 工具（进程 cwd 由 MCP 配置管理）、bang 路径（已修）、trust / permission 判定输入、session header / prompt env。

## Capabilities

- `agent-tools`：新增一条 requirement（工具执行相对会话工作区解析）+ `.feature` 可执行场景。

## Impact

- 受影响代码：protocol 工具端口、agent 工具批构造、infra 内置工具实现。
- 兼容性：Pre-0.0.1 无外部客户；`XyToolCtx` 构造点一次性改齐，不留双解析路径。
- 测试边界（复用既有 harness，不另发明 seam）：BDD `agent-tools.feature` 经 `XyTool::execute(&ctx, args)` 公共边界驱动（背景步骤已有临时工作目录）；单测覆盖 ctx 默认回落与 ReAct 冻结传递。

## Further Notes

- 设计取舍见 `design.md`：XyToolCtx 携带 vs per-driver 工具构造绑定，已拍板前者。
- bang 同类修复参考：commit `cab5ee6c`（`BashExecOpts.cwd` + Driver 传 `agent.cwd()`），回归 `bash_unary_runs_in_client_workspace` / `cwd_option_spawns_shell_in_workspace`。
