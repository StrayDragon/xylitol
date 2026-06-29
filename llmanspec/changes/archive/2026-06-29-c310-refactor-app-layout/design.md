# c310-refactor-app-layout — Design

## 1. 目标与非目标

**目标**：让 `app/` 的 seam（跨 surface 共享特权模块）与 surface（自洽入口）在结构上显式分离；修正 print 与 server-subcommand 的分类错位。

**非目标**：
- 不引入 Cargo workspace（沿用 c290 的单 crate 决策）。
- 不改变任何运行时行为、不改变公开 API 语义（仅迁移模块路径）。
- 不引入 `surfaces/` + `core/` 双层分组——当前 5 surface + 2 seam 规模下，`surfaces/` 一层是过度组织（见 §3）。

## 2. 设计决策

### 2.1 为什么 seam 不能"丢给其他模块"

`composition.rs` 是 **HC-1 组合根**——它同时 `use crate::agent::facade::Agent` 与 `use crate::infra::{bash_exec, session, provider, tools}`，而 `agent/` 被 arch_guard 硬约束为零 infra 导入（`agent_does_not_import_infra_in_production`）。因此 composition **必须**留在"唯一允许同时依赖 agent+infra 的层"= app 层；迁入 agent/infra/runtime_protocol 任一层都会触发分层不变式失败。`driver.rs` 同理（依赖 `agent::facade`，且 `RemoteDriver` 依赖 `app::server::ws` frame 类型，迁入下层会反向依赖）。结论：seam 是 app 层的原生职责，无法外迁，只能在 app 层内分组。

### 2.2 为什么用 `core/` 子目录而非扁平 + `pub(crate)`

考虑过的替代方案及其否决理由：

| 方案 | 否决理由 |
|---|---|
| 扁平 + `pub(crate)` + 注释分区 | 信息量够但视觉区分弱；`core/` 目录比 mod.rs 注释更自文档化，且 arch_guard 的 `exempt_prefixes` 天然支持前缀豁免 |
| `app/shared/` | "shared" 是泛化标签，稀释语义；`core` 更精确表达"app 层内核（组合根 + 边界）" |
| 文件名 `_composition.rs` / `_driver.rs` | Rust 中 `_` 前缀是语言级"故意忽略/压 dead_code"语义，用于**被高频使用**的核心文件语义完全相反；会逃过 dead_code 检查，误导读者 |
| `surfaces/` + `core/` 双层 | 5 surface 规模下 `surfaces/` 增加无谓路径深度（`crate::app::surfaces::cli`），`shared`/`core` 二选一已足够 |

选定 `core/`：命名精确（app 内核）、路径迁移成本可控（~6 处 `use`）、arch_guard 用 `exempt_prefixes = [..., "core/"]` 一行覆盖。

### 2.3 为什么 print 归 `cli/` 而非独立 surface

证据链：
1. **唯一调用方**：`rg "run_print"` 全仓仅 `cli/mod.rs:476` 一处。
2. **同类已存在**：`cli/provider_guidance.rs` 的模块 doc 自述 "pure CLI-surface presentation"，print 是同类呈现层。
3. **dispatch 语义统一**：CLI 解析后分派到 `rpc` / `server` / `print`，print 是默认分支，本质是 CLI 的一个 mode。
4. **feature 门控顺带修正**：`print.rs` 在 `app/mod.rs` 原本**无** `#[cfg(feature="cli")]`，无脑始终编译；移入 cli 后继承门控。

### 2.4 为什么 server-subcommand 归 `server/`

`ServerSubcommand`（Run/Install/Stop）+ `run_server`（含 `ServerLock::probe` + `kill -TERM`）是 server 生命周期管理，与 `server/lock.rs` 同域。enum 与 impl **一起**搬进 `server/`，避免"实现进 server、类型留 cli"导致的依赖倒流。`CliCommand::Server` 变体本身留在 cli（解析职责），dispatch 缩为一行委托。

### 2.5 la2 豁免清单的语义修正（顺带修 drift）

现状存在 spec/test 漂移：`la2` 文本豁免清单列 5 项（cli/, server/, rpc.rs, composition.rs, print.rs，**无 driver**），但 arch_guard 的 `exempt_files` 实际含 `driver.rs`。本次 modify la2 时一并修正——把 driver 显式纳入"documented seams"清单并标注其"agent-only、infra-banned（见 la11）"约束，使 spec 文本与测试实现对齐。这符合 AGENTS.md "update old call sites ... in one pass"。

## 3. arch_guard 影响分析

`src/tests.rs::arch_guard` 三处需改：

```rust
// 改前
let exempt_prefixes = ["cli/", "server/", "tui/diff_review/"];
let exempt_files = ["driver.rs", "rpc.rs", "composition.rs", "print.rs"];
// app_driver_does_not_import_infra: scan("src/app/driver.rs", "crate::infra")

// 改后
let exempt_prefixes = ["cli/", "server/", "core/", "tui/diff_review/"];
let exempt_files = ["rpc.rs"];   // print 入 cli/ 前缀；composition/driver 入 core/ 前缀
// app_driver_does_not_import_infra: scan("src/app/core/driver.rs", "crate::infra")
```

- `core/` 进 `exempt_prefixes`：composition（合法 agent+infra）与 driver（合法 agent、infra 由专项测试禁）都通过通用扫描。
- `app_driver_does_not_import_infra` 路径改为 `core/driver.rs`，保留 driver 的 infra 禁令（la11）。
- 守卫注释的文件地图同步更新。

## 4. 验证策略

1. `cargo check --features cli` / `--features server` / `--features tui` 全过（验证路径迁移无遗漏）。
2. `cargo test --lib arch_guard` 全过（验证豁免清单/扫描路径同步正确）。
3. `cargo test --lib` + `cargo test --test bdd -- --test-threads=1` 全绿（零行为回归）。
4. `llman sdd validate c310-refactor-app-layout --strict --no-interactive` + `llman sdd validate --all` 全过（spec 与代码路径一致）。

## 5. 回滚

纯 `git mv` + 路径替换 + 测试代码同步，单 commit 可整体回滚。无数据/格式迁移。
