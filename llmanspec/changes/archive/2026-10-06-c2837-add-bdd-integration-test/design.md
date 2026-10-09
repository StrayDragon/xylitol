# Design — BDD 独立集成测试目标（编译隔离）

## 背景与证据

| 测量 | 数值 |
|---|---|
| `cargo test --lib --no-run`（含 BDD） | 43s |
| 临时禁用 `mod bdd` 后 | 20s |
| BDD 占 lib 测试编译 | ≈53% |
| 步骤宏骨架重复族 | 0（去重已极致） |
| 需路径改写 | ≈500 处 `crate::<外部>` + 15 处 `crate::tests::bdd` |

## 目标架构

```
tests/bdd.rs          ← 新增集成测试 target（cargo test --test bdd → 现在 946 场景真实执行）
  └── #[path] mod bdd
        └── suite.rs  ← 保持现有全模块树（bindings_*/steps_*/fixtures/helpers/prelude）
```
lib 侧：`src/tests.rs` 只保留 `mod support` / `mod inner_*`；`src/lib.rs` 三模块可见性升格。

## 关键决策

### D1：可见性升格面（`agent` / `infra` / `utils`：`pub(crate)` → `pub`）
- **理由**：BDD steps 大量经 `crate::agent::*`、`crate::infra::*`、`crate::utils::*` 访问内部；独立 target 只能访问 `xylitol` 的 pub 项。
- **风险**：对外 API 扩张（此前这些模块对库用户不可见）。纯可见性提升，无行为变化；不 doc(hidden)，模块头注释说明升格动机（BDD 集成 target 可达性），保持透明。
- **备选**：`#[cfg(test)] pub mod` 条件导出——会引入 lib 测试配置才 pub 的奇异面，弃用；直接 pub 语义最干净。

### D2：target 内模块引用（`crate::tests::bdd` → `crate::bdd`）
- suite 内 `mod bindings_*` 为相对路径声明，不动；只有显式 `crate::tests::bdd` 引用需改（15 处）。

### D3：外部引用（`crate::<外部模块>` → `xylitol::<外部模块>`）
- 机械替换 + 编译器兜底（遗漏即编译错误，逐个清）。

### D4：qa 链路
- nextest `default-filter = "not binary(lab_responses_prompt_cache)"` 不排除新 target → `binary(bdd)` 自然入 `just test` 矩阵。
- `test-qa-gate` 规则不绑定具体 target 挂载位置 → 无需改 qa 规则；验证以 `just qa` 全绿 + 矩阵含 bdd 为准。

### D5：specs 变更
- `test-bdd` r37：文本不变（`cargo test --test bdd` 现落地为真），verified-by 更新为 `tests/bdd.rs` 与 `src/tests.rs`。
- 新增 r1913：BDD MUST 由独立集成测试 target 承载，MUST NOT 挂回 lib `cfg(test)`（编译隔离不变量）。带一个可执行场景（复用「读取 BDD 套件接线」步骤对）。

## 验证计划

1. `cargo test --test bdd` 946 场景全绿（r37 名实相符）。
2. `cargo nextest run --workspace --all-features --exclude xylitol-tui` 矩阵确认含 `binary(bdd)` 且全绿。
3. `just qa` 全量绿（含 test-tui / test-live-provider / doc-check / lint）。
4. `check_bdd_literal_bytes.py --check`（尖括号字面量门禁不受迁移影响）。
5. `llman-sdd validate --strict`（specs + changes 门）。
6. 编译基线复核：`cargo test --lib --no-run` 回落到 ≈20s。
