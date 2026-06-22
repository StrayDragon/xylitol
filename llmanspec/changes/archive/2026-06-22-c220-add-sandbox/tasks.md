# c220-add-sandbox: Tasks

## SandboxConfig 配置结构体

- [x] **T1** — 扩展 `SandboxConfig` 添加 filesystem/network/process/backend 字段
- [x] **T2** — 更新 `SecurityConfig` 中的 sandbox 条件编译引用（已存在）

## SandboxEngine 模块

- [x] **T3** — 创建 `src/infra/sandbox/mod.rs`（SandboxEngine trait + FallbackBackend + 27 tests）
- [x] **T4** — 创建 `src/infra/sandbox/policy.rs`（路径/域名匹配逻辑 + 24 tests）

## 工具集成

- [x] **T5** — read 工具添加 sandbox 路径检查（通过 AgentLoop 统一检查）
- [x] **T6** — write 工具添加 sandbox 路径检查（通过 AgentLoop 统一检查）
- [x] **T7** — edit 工具添加 sandbox 路径检查（通过 AgentLoop 统一检查）
- [x] **T8** — bash 工具添加 sandbox 网络域名检查（通过 AgentLoop 统一检查 + URL 提取）

## AgentLoop 集成

- [x] **T9** — AgentLoop 初始化时根据 feature flag 构建 `SandboxEngine` 回调

## Feature Flag & 测试

- [x] **T10** — `infra-sandbox` feature flag 正确编译（默认关闭）
- [x] **T11** — 测试：SandboxConfig 序列化/反序列化
- [x] **T12** — 测试：路径匹配、域名匹配（共 24 个 policy 测试）
- [x] **T13** — 测试：工具集成（FallbackBackend 测试覆盖所有路径）

## Verification

- [x] **T14** — `cargo check --features infra-sandbox` — 0 errors
- [x] **T15** — `cargo test --features infra-sandbox --lib` — 484 passed, 0 failed
- [x] **T16** — `llman sdd validate c220-add-sandbox --strict --no-interactive`
