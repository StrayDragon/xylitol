# c05-init-skeleton Tasks

## 1. 骨架搭建

- [x] 重写 `Cargo.toml`：保留现有 profile 配置，添加 `[features]` 段（default 含 `ui-tui`, `infra-session`, `ui-review`；共 13 个可选 feature）
- [x] 重写 `src/lib.rs`：声明 agent/、infra/、interface/ 三层模块，feature-gated 模块用 `#[cfg(feature = "...")]` 守卫，built-in 模块无守卫
- [x] 重写 `src/main.rs`：最小 `fn main() {}` 占位
- [x] 创建 `src/agent/mod.rs` + 占位子模块（loop.rs, planner.rs [feature = "agent-planning"], model.rs, tools/mod.rs [built-in]）
- [x] 创建 `src/infra/mod.rs` + 占位子模块：config/ [built-in], hooks/ [built-in], security/ [built-in], lsp/ [feature = "infra-lsp"], dap/ [feature = "infra-dap"], session/ [feature = "infra-session"], skills/ [feature = "infra-skills"]
- [x] 创建 `src/interface/mod.rs` + 占位子模块：cli/ [built-in], print.rs [built-in], acp.rs [feature = "infra-acp"], tui/ [feature = "ui-tui"]
- [x] 创建 `configs/config.schema.json` 空占位（`{}`）
- [x] 删除原 `src/lib.rs` 中的 `place_holder()` 函数（已被新 lib.rs 替换）

## 2. 验证

- [x] `cargo check` — 仅 default features 编译通过
- [x] `cargo check --all-features` — 所有 features 编译通过
- [x] `just fmt` — 格式化通过
- [x] `just lint` — clippy 无警告
- [x] `llman sdd validate c05-init-skeleton --strict --no-interactive``
