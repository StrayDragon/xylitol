# Design — c405-add-tui-test-harness

> 本变更涉及多个测试基础设施的权衡决策，记录在此供后续维护者理解「为什么这样选」。

## 决策 1：复用现有 vte cell-grid harness，不引第三方 testlib

### 选项

| 方案 | 来源 | 结论 |
|---|---|---|
| **A. 复用现有 `tests/support/mod.rs`（vte）** | xy 自建 | ✅ 采用 |
| B. 引入 `terminal-testlib`（crates.io `ratatui-testlib`） | 第三方 | ❌ 拒绝 |
| C. 引入 `expectrl` / `rexpect`（expect 风格） | 第三方 | ❌ 拒绝 |

### 理由

- **方案 A 已具备全部核心能力**：`VirtualTerminal`（实现 `Terminal` trait + vte 解析 SGR/cursor/erase/OSC）+ `LoggingVirtualTerminal`（记录 write 字节断言差分契约）+ cell/cursor/viewport 查询。和 pi-tui 的 `@xterm/headless` harness、zellij 的 `Grid(vte)` 同源同思路。
- **方案 B 风险高**：`ratatui-testlib` 仅 8 star、2025-11 才建、版本发布不一致（crates.io 0.1.0 vs 仓库 0.6.0）、为 Bevy/Sixel 定制。它内部自带 termwiz 解析，会和 xy 已有的 vte harness 形成两套屏幕模型。
- **方案 C 不做渲染断言**：expectrl/rexpect 把终端当字节流，不懂 cell grid。对 TUI 差分渲染断言无能为力。

### 后果

- 第 5a 层（portable-pty）需要把 `tests/support/` 的解析能力对 crate 外可见——见决策 3。
- 不引入 `termwiz` / `vtparse` 第二套解析器，保持单一屏幕模型。

## 决策 2：tmux vs portable-pty 的分工

### 背景

两者都能做 E2E，但能测的东西有实质差异。

### 分工

| 层 | 工具 | 测什么 | 为什么 |
|---|---|---|---|
| 5a（主力）| portable-pty | crossterm 真实事件解析（Ctrl+方向键、Alt+方向键、bracketed paste）+ 整链路 smoke | in-process 读字节流喂给 vte harness，断言精确到 cell；快（无 spawn 开销之外的终端模拟器）|
| 5b（冒烟）| tmux（3-5 条）| 真终端兼容性 + 颜色 SGR 回归 | 最接近人眼（真实终端模拟器渲染），但慢（1-3s/case）+ tmux 规范化 ANSI（断言要宽松）|

### 为什么不全用 tmux

- 慢：spawn 子进程 + 轮询 capture-pane，每 case 1-3s。跑 50 个就 2 分钟，不适合主矩阵。
- 断言精度受限：tmux 会规范化 ANSI（如 `\x1b[0m` 改成 `\x1b[39m`），cell-by-cell 断言脆弱。只能用「包含某文本」+ snapshot（带 filter）。
- flaky：tmux 无「渲染就绪」信号，必须轮询屏幕内容，CI 上慢机器易超时。

### 为什么不全用 portable-pty

- 绕过了真实终端模拟器：颜色/宽度在真终端里的兼容性回归（如某终端不支持 256 色、CJK 宽度差异）测不到。
- tmux 作为「最接近人眼」的一层，保留少量冒烟有独立价值。

### 后果

- 两套 E2E driver 都要维护，但职责不重叠（5a 精确快速，5b 真终端宽松慢）。
- 两套都标 `#[ignore]`，不进默认 `cargo test`。

## 决策 3：`tests/support/` 模块对 E2E 测试的可见性

### 问题

`packages/xylitol-tui/tests/support/mod.rs` 是 crate 内测试支持。workspace 顶层的 `tests/tui_e2e/`（集成测试）需要复用其中的 `VirtualTerminal`（解析 portable-pty 读到的字节流）。

### 选项

| 方案 | 做法 | 结论 |
|---|---|---|
| A. 把 support 编进 lib（feature gate）| `#[cfg(feature = "test-support")] pub mod support` | ❌ 污染生产 lib |
| B. 在 `tests/tui_e2e/` 复制最小 vte 解析 | 重复代码 | ❌ 维护负担 |
| C. 把 support 提为独立 dev-crate | `xylitol-tui-testsupport` crate | ⚠️ 过度工程化 |
| **D. 在 `tests/tui_e2e/` 直接 `use xylitol_tui::...`** | 仅复用已公开的 `Terminal` trait + 自己写薄解析 | ✅ 采用（若 support 内类型已公开）|

### 倾向 D，但需在实现时确认

- `xylitol-tui` 的 `lib.rs` 已公开 `Terminal` trait、`CrosstermTerminal`。
- `tests/support/mod.rs` 的 `VirtualTerminal` 是 **test-only**，不在 lib 里。
- **落地策略**：第 5a 层实现时，在 `tests/tui_e2e/` 内写一个最小的 vte 解析 wrapper（~50 行，复用 `vte` crate 直接解析），或把 `VirtualTerminal` 的核心解析逻辑抽成一个 `pub mod test_support` 在 lib 里（仅 `#[cfg(any(test, feature = "test-support"))]` 暴露）。**具体选哪个在第 5a 阶段实现时决定**——不提前锁死，避免空转。

## 决策 4：Clock 抽象的形态

### 背景

paste-burst（同步，用 `Date.now()`/`Instant::now()`）和 autocomplete debounce（pi 用 `setTimeout`，即 async）的时间源不同。

### 决策

- **同步逻辑（paste-burst）**：trait `Clock { fn now(&self) -> Instant }`，`PasteBurst::new(clock)`。测试传 `MockClock { t: Instant }`，`advance(dur)` 推进。生产传 `SystemClock`。
- **async 逻辑（debounce）**：直接用 tokio 时间，测试用 `#[tokio::test(start_paused = true)]` + `tokio::time::advance`。不另造 async Clock trait——tokio 的 paused time 已是标准方案。

### 为什么不统一成一个 Clock

- paste-burst 的移植尚未发生，但它是同步逻辑（pi 源码用 `number` 时间戳）。强行套 tokio 会引入不必要的 async 污染。
- 两套时间源对应两种代码形态，各自用最自然的测试方式。

### 后果

- c405 只建 Clock trait 骨架 + 示例测试。paste-burst 移植（c410+）时才真正用到。
- debounce 的时序测试随 autocomplete 移植补，本变更只验证 `start_paused` 机制可用。

## 决策 5：snapshot 用 insta，不学 pi 刻意避开

### 背景

pi-tui 刻意不用 snapshot（TS 生态 ANSI 脆弱、无好的 review 工具）。ratatui 官方推荐 insta。

### 决策：引入 insta

- Rust 的 insta 有 `cargo insta review`（交互式 diff 审查）+ inline snapshot + filters（抹掉不稳定部分如时间戳）。
- 对 TUI 渲染回归（布局、颜色、换行变化）极有效——手写 `assert_eq` 对多行带样式输出痛苦。
- 与 `assert_eq` 混用：snapshot 管「整屏回归广度」，`assert_eq` 管「单 cell 精确性」。

### 后果

- 引入 `insta` dev-dependency + snapshot 文件（`.snap`）进版本控制。
- 接受 snapshot 需人工 `cargo insta review`（AGENTS.md 已规定「快照用 insta，接受前复核」）。

## 风险与缓解

| 风险 | 缓解 |
|---|---|
| portable-pty 测试在 CI flaky（spawn 时机）| 标 `#[ignore]` + 本地验证为主；CI 夜跑 |
| tmux 版本差异导致 capture 行为不一 | 固定 `TERM=xterm-256color` + 用「包含」断言而非精确匹配 |
| proptest 随机 case 偶发失败难复现 | proptest 自带 seed shrinking + 失败 case 自动记录到 `proptest-regressions/` |
| Clock 抽象空转（paste-burst 还没移植）| c405 只建 trait + 骨架测试，不写没用到的消费者 |

## 非目标（明确排除）

- 不移植任何 pi-tui 功能（editor 扩展、paste-burst、autocomplete 补全）——后续 c410+。
- 不替换 `src/app/tui/engine/`——后续变更。
- proptest 的 editor 不变量具体定义——留给 editor 移植变更。
