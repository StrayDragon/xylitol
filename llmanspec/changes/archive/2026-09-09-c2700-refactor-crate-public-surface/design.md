# Design: 收窄 crate 公开面

> Designed / pre-start。不执行 `change start`、Specs landing、代码修改，直到批次 review 认领。

## 1. 目标

外部编译单元能 `use` 的 xylitol API = `embed` + `src/lib.rs` 已精选 `pub use` + 必要的 protocol 根类型。`agent` / `infra` 实现细节只对主 crate 可见。

## 2. 代码事实

| 事实 | 影响 |
|---|---|
| `src/lib.rs` 已 `pub use` Driver、HostClient、Xy* ports/errors/events | 收口后这些仍是稳定面 |
| `embed.rs` 写明不要把 `infra::*`、`agent::capabilities::*` 当稳定；又承认 `BootstrappedAgent::agent` 泄漏 `AgentRuntime` | 本 change 修文档与可见性；Runtime 公开面收缩交给 `c2705` |
| 测试大量 `xylitol::infra::event::EventBus`、`xylitol::agent::…` | 改为 `crate::`（同 crate 测）或 `embed` / Driver（跨 crate 测） |
| `src/AGENTS.md`：组合根才同时 import agent+infra；禁止 grep 元测试卡 import | 可见性用 rustc，不用元测试 |

## 3. 可见性矩阵

| 模块 | 本 change 后 | 谁可以 import |
|---|---|---|
| `embed` | `pub` | 外部 + 仓内 |
| 根 `Xy*` / `XyDriver` / `HostClient` | `pub` | 外部 |
| `agent` | `pub(crate)` | 主 crate 组合根、agent 测 |
| `infra` | `pub(crate)` | 主 crate 组合根、infra 测 |
| `app` | 默认 `pub(crate)`；若 `embed` 必须 `pub use` 个别类型则只 re-export 那些 | TUI/CLI 仍在主 crate 内 |
| `protocol` 根精选 | `pub` | 与今日 `Xy*` 一致 |
| `protocol` 深子模 | 尽量 `pub(crate)`，避免第二稳定面 | 仓内 |
| `utils` | `pub(crate)` | 各层 |

`packages/xylitol-tui` / `xylitol-ai-bridge` 本来就不依赖主 crate，不受影响。

## 4. 迁移步骤（expand 不必，pre-0.0.1 直接破）

1. 把 `pub mod agent/infra` 改为 `pub(crate)`，编译，修所有 `xylitol::agent` / `xylitol::infra`。
2. `app`：外部若只需 `run()` 与 `embed`，`pub mod app` 也可 `pub(crate)`，保留 `pub async fn run`。
3. 扫 `tests/`、`tests/bdd` 的 `xylitol::` 深路径。
4. 更新 `embed.rs` / crate 根 rustdoc：删除「advanced embedding 可 reach 子模」的口子。
5. `BootstrappedAgent::agent` 字段：本 change **可**先改 `pub(crate)`；完整 Runtime API 收缩仍是 `c2705`。

## 5. 非目标

`XyDriver` 方法数量、Command 枚举、EventBus 语义、compaction 配置形状。

## 6. 验证

- `cargo test --workspace --all-features` 与 `just qa` 绿。
- 仓外最小 smoke：`use xylitol::embed` + `XyDriver` 能编；`use xylitol::infra::…` 必须失败（可用独立小 crate 或 doctest `compile_fail`）。
- 不新增 `Xy*` port。
