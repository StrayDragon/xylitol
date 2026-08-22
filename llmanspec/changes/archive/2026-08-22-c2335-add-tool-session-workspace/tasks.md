# Tasks

测试边界（复用既有 harness，不另发明 seam）：
- **BDD**：`agent-tools.feature` 经 `tests/bdd/bindings_agent_tools.rs` 驱动 `XyTool::execute(&ctx, args)` 公共边界——新场景挂同一边界。
- **单测**：infra 工具模块内（tempdir + ctx 工作区）、agent `tool_exec` / ReAct 冻结传递、protocol ctx 默认回落。

## 1. Specs landing（Branch binding 后、apply 前）

- [x] 1.1 `change start` 绑定 `sdd/c2335-add-tool-session-workspace`
- [x] 1.2 live specs：`agent-tools` toon 新增 requirement（模型侧工具相对路径与 bash 子进程 cwd = 运行时注入的会话工作区；未注入回落 Host 进程 cwd）；`.feature` 新增 `@req` 可执行场景（工具在指定工作区解析相对路径）
- [x] 1.3 commit Specs landing；结构过闸（`llman sdd validate agent-tools --no-check` 绿）

## 2. 协议缝与运行时穿线

- [x] 2.1 [blocked-by: 1.3] protocol：`XyToolCtx` 增工作区字段与 `with_workspace` 构造子；默认构造回落进程 cwd（单测断言默认回落 + 显式覆盖）
- [x] 2.2 agent：`ToolExecEnv` 增 workspace；`run_one` 构造 ctx 时携带；ReAct 用 run 冻结 cwd 填充（单测/现有测试不红）

## 3. bash 工具双路径

- [x] 3.1 [blocked-by: 2.2] infra：非流式路径 `BashOperations::execute` 签名增 cwd 并 `.current_dir()` 生效；MockBash 等调用点一次改齐
- [x] 3.2 流式路径传 `BashExecOpts.cwd = Some(ctx.workspace)`（对齐 bang 修复 `cab5ee6c`）
- [x] 3.3 回归单测：tempdir 为 ctx 工作区时 `pwd` 落该目录；未设工作区回落进程 cwd

## 4. 文件类内置工具

- [x] 4.1 [blocked-by: 2.2] `path_utils::resolve_to_cwd` 改吃显式 base（ctx 工作区）；ls/grep/find 接线
- [x] 4.2 read/write/edit：相对路径先 join ctx 工作区再交 fs / `FileMutationQueue`（键为解析后绝对路径；patch.rs 为 edit 的匹配 helper，非模型可调用工具）
- [x] 4.3 单测：write→read 经同一 tempdir 工作区往返成功；绝对路径行为不变

## 5. BDD 场景与收口

- [x] 5.1 [blocked-by: 3.3, 4.3] `agent-tools.feature` 新场景挂 `@req`：背景给临时工作区，bash/文件工具以该工作区执行并断言落点
- [x] 5.2 bindings 接线；`cargo test --test bdd` 绿
- [x] 5.3 attach 护栏（remote 测试样式）：writer 槽物化于 tempdir 后，模型侧工具经完整 driver 执行落在该目录（对齐 `bash_unary_runs_in_client_workspace` 先例）
- [x] 5.4 lib + bdd 全量绿；勾根 `_TUI_MIGRATED_TODO.md` P1「A7 后续」项
