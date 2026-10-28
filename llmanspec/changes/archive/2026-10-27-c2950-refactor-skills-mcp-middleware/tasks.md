# Tasks: c2950

## 1. skills 展开前移（内置项）

- [x] 1.1 把 `expand_skills_in_agent_messages` 调用移到 `before_model` 执行之前（`react/mod.rs`）。
- [x] 1.2 确认内置展开不进入 `LoopMiddlewareRegistry`、不产生 `loop.middleware` span。
- [x] 1.3 单测：展开结果不变；空 skills no-op；第三方 `before_model` 能观察到展开后的 messages。

## 2. `r2931` 口径澄清

- [x] 2.1 `agent-runtime` 的 `r2931` 改为「未配置**第三方可注册**中间件时链为空；内置项不计入链、不产生 span」。
- [x] 2.2 补一条场景钉住「内置项不计入链」。

## 3. MCP 结果截断

- [x] 3.1 官方中间件侧实现 `after_tool` 截断（单段 Text 超 50 KiB 截断并加提示）。
- [x] 3.2 含 Image 等多分片时不截断（遵守 `r2929`）。
- [x] 3.3 MCP bridge 不改（仍全量返回）。
- [x] 3.4 单测：截断边界、多分片保真。

## 4. BDD

- [x] 4.1 基线等价场景（无第三方 vs 有第三方只读，messages 一致）。
- [x] 4.2 MCP 截断场景（超限被截断带提示）。
- [x] 4.3 绑定并跑通。

## 5. 文档与验证

- [x] 5.1 `src/AGENTS.md`「Loop 中间件」段补「内置项 vs 可注册项」分层。
- [x] 5.2 `cargo test --all-features --test bdd` 绿。
- [x] 5.3 `llman-sdd validate --all --strict` 绿。
- [x] 5.4 `just qa` 绿。
