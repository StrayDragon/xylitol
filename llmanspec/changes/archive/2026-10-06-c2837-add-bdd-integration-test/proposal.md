---
depends_on: []
branch: sdd/c2837
base_branch: main
base_sha: 4aea70bb6062b69414807531387e3a657f4cc138
---

## Why

编译面隔离证据（实测）：
- `cargo test --lib --no-run` 含 BDD 模块 43s；临时禁用 BDD 模块 20s——**BDD 占 lib 测试编译 53%**。
- BDD（946 场景宏 + 1898 步骤宏）经 `src/tests.rs` `mod bdd` 挂入 lib `cfg(test)`，任何 lib 测试改动都连带 2785 处 proc-macro 展开与整棵 `tests/bdd/` 树编译。
- 步骤宏骨架重复族实测为 0（steps 层已极致去重），「合并宏族降展开」路径被证伪；唯一的编译面杠杆 = **把 BDD 迁出 lib，成为独立集成测试 target**。
- 附带：`test-bdd` 规则 r37 声称「BDD 全量通过（cargo test --test bdd）」，但当前 **不存在 `--test bdd` target**（实测 0 测试）——规则悬空，与实现的真实载体（lib 内 bdd 模块）不符。此 change 使 r37 **名实相符**。

## What Changes

- 新增集成测试入口 `tests/bdd.rs`：`#[path = "bdd/suite.rs"] mod bdd`（保持 `tests/bdd/` 文件布局与模块树不动）。
- `src/tests.rs`：移除 `mod bdd` 挂载（保留 support / inner 挂载）。
- `src/lib.rs`：`agent` / `infra` / `utils` 由 `pub(crate) mod` 升为 `pub mod`（纯可见性扩张，零行为变化；供 BDD target 经 `xylitol::` 访问）。
- 路径改写（机械）：`tests/bdd/**` 内 `crate::tests::bdd` → `crate::bdd`；`crate::<外部模块>` → `xylitol::<外部模块>`。
- specs（`test-bdd`）：r37 verified-by 更新至 `tests/bdd.rs`；新增规则 r1913「BDD 独立测试目标承载（编译隔离不变量）」。
- 验证面：`cargo test --test bdd`（r37 落地）、`just qa` 全绿且 nextest 矩阵含 `binary(bdd)`（qa 链路保持）、`check_bdd_literal_bytes`、`llman-sdd validate --strict`。

## Out of Scope

- 不换基座（继续 rstest-bdd 0.6.0）。
- 不做 BDD 内容治理（可读性档 1，另立 change）。
- 不求子 package 额外拆分（xylitol-tui / xylitol-ai-bridge 无 rstest-bdd 挂载，其 features 由本 target 消费）。
