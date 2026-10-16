# 测试分层与 `lab_` 约定

## 分层（业务分工）

| 层 | 载体 | 职责边界 |
|---|---|---|
| BDD 端到端 | `tests/bdd`，rstest-bdd，中文 Gherkin | 编排级行为验收；场景真源在 `llmanspec/specs/<capability>/` |
| 单测 | `src` 内联 | 纯数据 / 组件边界；**不重复全链路** |
| 快照 | `tests/snapshots`（insta） | 整屏布局 / 序列化形状回归 |
| wire 契约 | wiremock / 脚本化 SSE mock | 假网关；含 must-not-send 断言 |
| 真网关 lab | `lab_` 前缀，无配置自动 SKIP | 只有真环境才有的行为 |
| 真终端 E2E | `#[ignore]`，`just qa-e2e` 才跑 | 真 PTY / tmux 字节路径 |

原则：**BDD 管端到端编排；单测管纯数据 / 组件边界，不重复全链路。** 新功能先问「哪一层最能以有限注意力钉住行为」，避免堆量。

## 试验 / 真实网关验证命名（`lab_`）

试验性证据与真实网关探针 **统一前缀 `lab_`**（不再用 `evidence_` / `experiment_` / 试验性 `live_` 二进制名）：

| 形态 | 约定 | 是否进 `just qa` |
|---|---|---|
| `packages/*/examples/lab_*.rs` | 人跑维护 lab（`cargo run -p … --example lab_…`） | **否** |
| `packages/*/tests/lab_*.rs` | 可连接真实网关的 lab 二进制；是否纳入门禁看 just 接线 | **仅**已接线者（见 `qa-test-live-provider`） |
| `#[test] fn lab_*`（无 `#[ignore]`） | 门禁内契约实验（无网或假网） | **是**（随 crate 测） |
| `#[ignore]` 的 `lab_*` 测试 / `src/**/lab_*.rs` lab 模块 | 人跑维护 lab（模块 doc 给命令） | **否** |

配置文件名 `live-provider.yaml` / recipe `qa-test-live-provider` 保留「live 网关」语义，与代码符号前缀无关。新试验 **MUST** 用 `lab_`；**禁止**再引入 `evidence_` / `experiment_` 前缀。

## 并行与进程级全局状态纪律

- 新测试**禁止**无必要引入新的进程级可变全局；能注入 / 局部化则注入。
- 必须串行时用 `#[serial(<domain>_global)]`，**禁止**默认空组把无关域捆在一起。
- 主门禁 `just qa` 走 nextest（一测一进程）：勿假设 in-process `#[serial]` 跨测生效。无固定端口 / 共享非 temp 路径等外部共享资源时**不**配置 `[test-groups]`；出现再开。
- `cargo test` 回退与 `just test-tui` 仍依赖 in-process 命名组。
- 配置 / trust / session 测走注入与 tempfile；勿改 `HOME`、勿指到开发者真实 `~/.xylitol/sessions`。产品路径勿无必要写包级 `GLOBAL_KEYBINDINGS`。

## 时序

**禁止 `thread::sleep`** 做时序测试：同步用 `Instant` 注入 / `MockClock`；async 用 `#[tokio::test(start_paused = true)]`。

## TUI 验证

TUI 专项分层（按键→状态 / snapshot / 时序 / 真终端 E2E）与落点：`references/tui-harness.md` + `packages/xylitol-tui/AGENTS.md`「验证」。
