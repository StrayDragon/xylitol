# c05-init-skeleton Tasks

## 1. 骨架搭建

- [ ] 重写 `Cargo.toml`：保留现有 profile 配置，添加 `[features]` 段（default 含 `ui-tui`, `infra-session`, `ui-review`；共 12 个可选 feature）
- [ ] 重写 `src/lib.rs`：声明 agent/、infra/、interface/ 三层模块，feature-gated 模块用 `#[cfg(feature = "...")]` 守卫，built-in 模块无守卫
- [ ] 重写 `src/main.rs`：最小 `fn main() {}` 占位
- [ ] 创建 `src/agent/mod.rs` + 占位子模块（loop.rs, planner.rs [feature = "agent-planning"], model.rs, tools/mod.rs [built-in]）
- [ ] 创建 `src/infra/mod.rs` + 占位子模块：config/ [built-in], hooks/ [built-in], security/ [built-in], lsp/ [feature = "infra-lsp"], dap/ [feature = "infra-dap"], session/ [feature = "infra-session"], skills/ [feature = "infra-skills"]
- [ ] 创建 `src/interface/mod.rs` + 占位子模块：cli/ [built-in], print.rs [built-in], rpc.rs [built-in], tui/ [feature = "ui-tui"]
- [ ] 创建 `configs/config.schema.json` 空占位（`{}`）
- [ ] 删除原 `src/lib.rs` 中的 `place_holder()` 函数（已被新 lib.rs 替换）

## 2. 验证

- [ ] `cargo check` — 仅 default features 编译通过
- [ ] `cargo check --all-features` — 所有 features 编译通过
- [ ] `just fmt` — 格式化通过
- [ ] `just lint` — clippy 无警告
- [ ] `llman sdd validate c05-init-skeleton --strict --no-interactive`
