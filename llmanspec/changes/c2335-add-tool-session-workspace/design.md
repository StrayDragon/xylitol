# Design — 工具执行面工作区穿法

## 决策：XyToolCtx 携带（已拍板），不做 per-driver 工具构造绑定

| | XyToolCtx 携带（选定） | per-driver 工具构造绑定 |
|---|---|---|
| 工具状态 | 保持无状态，执行环境经 ctx 注入（与 cancel / output_tx 同类） | 每个文件/bash 工具实例持有 workspace，变有状态 |
| 冻结语义 | ReAct 从 run 冻结配置取 cwd，与 B3 run 绑定同构；run 内工具环天然一致 | 需在物化时烧入；switch_session / set_cwd 后要重建工具集 |
| MCP reload 交互 | `builtins_for_reload` 重建无需感知 cwd | reload / settle 两条重建路径都要穿 cwd，易漏 |
| 测试 | 构造 ctx 即可单测 | 需起完整 driver 装配才能测 |

## 数据流

```
Host materialize_writer_at → agent.set_cwd(workspace)          （A7 P0 已落地）
ReAct pending_root_stream  → cwd = inner.cwd()（冻结进 run）     （已有局部变量）
  └─ ToolExecEnv 增 workspace 字段
       └─ run_one 构造 XyToolCtx::with_cancel(..).with_output_tx(..).with_workspace(cwd)
            ├─ bash: 流式 → BashExecOpts.cwd = Some(workspace)
            │        非流式 → BashOperations::execute 增加 cwd 参数（内部 .current_dir）
            ├─ ls/grep/find: resolve_to_cwd(path, base=ctx.workspace)
            └─ read/write/edit/patch: 相对路径先 join(ctx.workspace) 再交 fs/mutation queue
```

## 关键点

- **默认回落**：`XyToolCtx` 默认构造（测试 / MCP assemble 探针）回落 Host 进程 cwd——print/库嵌入同进程行为不变；显式 `with_workspace` 才覆盖。不留双语义 API。
- **`BashOperations` trait 签名**：加 `cwd: &Path` 参数。该 trait 是 infra 内部测试缝（MockBash 在 cfg(test)），一次性改调用点，不加默认方法兼容层（Pre-0.0.1 卫生）。
- **mutation queue 键**：write/edit 先解析为绝对路径再进 `FileMutationQueue`，per-path 序列化键跨会话不串。
- **MCP 工具**：`mcp__*` 经 adapter 执行在 MCP server 进程侧，cwd 归 MCP 配置管，本变更不触碰。
- **hook 可见性**：模型侧 bash pre/post-spawn hook 与脚本 hook bus 的载荷不含 cwd 字段；如需观测属新合约，不在本刀。

## 权衡记录

- 曾考虑把解析下沉到「所有工具统一入口」包装层——违反 t12（ToolSet 无运行时 wrap API）且给非文件工具白增开销；按工具就地解析更贴现状。
- 不改 permission target 解析（`permission_target(name, &args)` 吃原始 args）：闸的语义是「模型请求了什么路径」，与落盘解析分属两层；若未来要按解析后绝对路径闸，另立合约。
