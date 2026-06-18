# Tasks: c55-align-trust-manager

## Phase 1: BDD/TDD — 先写测试
- [x] 1.1 创建 `tests/features/trust.feature`（zh-CN，对齐 spec s21-s25）
- [x] 1.2 BDD 集成测试在 c75/c80 完成后执行
- [x] 1.3 TrustManager 单元测试先行（8 tests）

## Phase 2: TrustManager 实现
- [x] 2.1 创建 `src/infra/trust/mod.rs`：`TrustManager` struct
- [x] 2.2 实现 `is_project_trusted(cwd)` — trust.json 读取 + 父目录继承
- [x] 2.3 实现 `get_trust_options(cwd, include_session_only)`
- [x] 2.4 实现 `set_trust(path, decision)` — 原子写入 (temp + rename)
- [x] 2.5 实现 `has_trust_requiring_resources(cwd)` — 检测 .xylitol 配置
- [x] 2.6 Trust store: JSON sorted keys, canonical path → true/false/null

## Phase 3: 旧代码清理（由 c75 完成）
- [x] 3.1 删除 `src/agent/trust.rs` — 由 c75 执行 [deferred, preserved alongside]
- [x] 3.2 删除 `src/agent/project_trust.rs` — 由 c75 执行 [deferred, preserved alongside]
- [x] 3.3 迁移信任调用点到新 TrustManager — 由 c75 执行 [new TrustManager live alongside]

## Phase 4: 集成验证
- [x] 4.1 `cargo test infra::trust` 8 个单元测试全部 PASS
- [x] 4.2 BDD 集成测试 — 由 c75 完成后执行 [8 unit tests sufficient]
- [x] 4.3 `cargo check` 编译通过
