# T4/T5 thoughtdag 跨仓适配点清单（2026-11-21）

> 对象仓：/Users/l8ng/Projects/xylitol-thoughtdag（main @ 3efb6c7，与 origin 同步，树干净）。
> 依据：c3410 research/04-thoughtdag-coupling.md 耦合图 + 本仓分支实跑 `cargo check --workspace --all-targets`。

## T4（session_store 收窄 + 九件删除）后

实跑结果：`cargo check --workspace --all-targets` 绿（仅既有 `unused variable: workspace` 警告）。
thoughtdag 未消费 `XyDriver::session_store()` 与被删九件（剪贴板三件 / attach 六件）→ **无需适配**。

## T5（XySessionOps 升格）后

实跑结果：xtd-core 编译失败，5 处固有方法调用缺 trait 作用域（预期编译期适配点）：

| 文件 | 行 | 调用 |
|---|---|---|
| crates/xtd-core/src/kernel.rs | 278 | `driver.switch_session(session_id)`（run_turn 内会话对齐） |
| crates/xtd-core/src/kernel.rs | 324 | `driver.list_sessions()` |
| crates/xtd-core/src/kernel.rs | 331 | `driver.load_session_entries(session_id)` |
| crates/xtd-core/src/kernel.rs | 338 | `driver.switch_session(session_id)` |
| crates/xtd-core/src/kernel.rs | 349 | `driver.fork_session(entry_id, position)` |

最小适配：kernel.rs:7 `use xylitol::embed::{...}` 增补 `XySessionOps`（本仓已在 `xylitol::embed` 入口 re-export 该 trait）。xtd-web 经 XtdKernel wrapper 方法间接调用，不直接触碰 driver 固有方法，无需改动。

## 错误原文摘录（T5 后首次实跑）

```
error[E0599]: no method named `switch_session` found for struct `XyInProcessDriver` in the current scope
error[E0599]: no method named `list_sessions` found for struct `XyInProcessDriver` in the current scope
error[E0599]: no method named `load_session_entries` found for struct `XyInProcessDriver` in the current scope
error[E0599]: no method named `fork_session` found for struct `XyInProcessDriver` in the current scope
error: could not compile `xtd-core` (lib) due to 5 previous errors
```
