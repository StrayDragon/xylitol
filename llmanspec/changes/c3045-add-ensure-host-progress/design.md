# Design — 确保 Host 等待过程可见

## 目标

产品 TUI 在本机 ensure Host 的等待窗口内，TTY 上给出一行短进度；Host 已 Ready 时不闪等待面；失败时清掉进度并落既有可操作诊断。等待面必须在进入产品 TUI 画布 / `TerminalGuard::enter` 之前，不得提前 raw / alt-screen。

## 现状

- `src/cli.rs:99-120`：`want_tui` 且 `should_ensure_host` 时先 `ensure_host_from_config(...).await`，失败 `eprintln!` 后返回；此时尚未进 TUI。
- `crates/xylitol-kernel/src/attach.rs:534-558`：`ensure_host_with` 先 `inspect_live`，Ready 直接返回；否则 spawn 后 `wait_until_ready`。
- `wait_until_ready`：50ms 轮询 `/healthz`，默认 `READY_WAIT = 30s`（`attach.rs:470`）。
- `crates/xylitol-app-tui/src/lib.rs:148`：driver attach 成功后 `TerminalGuard::enter()`；ensure 阶段没有终端 UI。
- 现有诊断：`ready_timeout_message` / `spawn_failed_message` / `auth_required_message` / `classify_running`。

## 决策

- 形态：产品画布前的**一行 TTY spinner**，不是 boot 场景、不是假 Busy / 假 Reloading。
- 文案：固定短词 `Starting host`；与 status 槽 `Reloading` / `Assembling` 同形（accent spinner + 贴左短词）。
- 相位：MVP 只一句 `Starting host`；listening / assembling / ready 相位后置。
- 延迟启动：150ms。ensure 在 150ms 内完成（已 Ready 或极快 spawn）则不输出，避免闪。
- 帧间隔：80ms；ASCII/Unicode 帧均可，优先复用现有产品 Loader 形制，不另做 theme。
- 输出：stderr；`\r` 回写 + `\x1b[2K` 清行；成功/失败都清行，不留半行。
- 失败：清行后 `eprintln!` 既有诊断；不得只剩空白 / mux 垃圾。
- 取消：MVP 不提供自定义取消；等待期间 Ctrl+C 由终端 SIGINT 终止进程。不因进度面进入 raw mode。
- 覆盖范围：只覆盖 spawn→`/healthz` ready。已有 Host 但 WS 握手慢不在本 change；`--no-ensure-host`、显式非确保 `--attach`、print / 嵌入、非 TTY、`server` feature 关闭：无动画。
- 顺序：ensure 进度必须在 trust gate 之前结束；trust gate 仍走既有交互。
- `TerminalGuard::enter` 只能在 ensure 成功且 trust gate 通过后调用；不得提前。

## 状态机

```text
Idle
  └─ start_if_tty(label) + 150ms timer
       ├─ finish before timer → Idle（无输出）
       └─ timer fires → Spinning
            ├─ finish(success) → Clear
            └─ finish(error) → Clear + eprintln(diagnostic)
```

- `Spinning` 由 tokio task 每 80ms 写一帧；`finish` 取消 task 并清行。
- 写失败（stderr 关闭 / 非 TTY）静默降级，不影响 ensure 结果。

## API 形状（拟）

```rust
pub struct BootProgress { /* writer, label, task, started */ }

impl BootProgress {
    pub fn start_if_tty(label: &str) -> Option<Self>;
    pub fn finish(self);
}

pub async fn with_boot_progress<T>(
    label: &str,
    fut: impl Future<Output = T>,
) -> T;
```

- 放 `crates/xylitol-app-tui/src/boot_progress.rs`，经 `lib.rs` 导出；root `src/cli.rs` 使用。
- writer 泛化为 `impl Write` 以便单测；生产用 `std::io::stderr()`。
- 时间源注入 `Duration` / tick 间隔，单测不 sleep 真实 150ms（用可控 clock 或短间隔）。

## 接入点

`src/cli.rs`：

```rust
let progress = BootProgress::start_if_tty("Starting host");
let result = ensure_host_from_config(&url, config).await;
progress.finish();
match result { Ok(()) => {}, Err(msg) => { eprintln!("{msg}"); return Err(...); } }
```

- 只在 `should_ensure_host` 分支启动；`attach_preflight` 分支不启动。
- `progress.finish()` 在 trust gate 之前。
- 不改 `ensure_host_with` 的签名；MVP 不需要 progress callback。

## 已钉决策（propose）

- Spec 落点：在 `cli-entry` 新增需求块 `ensure-host-progress`（新 `@req`），不改 r1391 / r3065。理由：c3040 正在改这两个块，同块叠加会让两次 finalize 相互冲突；进度可见是独立产品合约。
- r3249 的 `starting local Host at <url> ...` 行保留；TTY 上 spinner 出现在其下一行（150ms 后），非 TTY 只留该行。不合并成一行，避免改 r3249 既有文案。
- 文案固定 `Starting host`；80ms 帧；stderr；`\r\x1b[2K` 清行；成功与失败都清行。
- `progress.finish()` 必须在 trust gate 之前；`TerminalGuard::enter` 不得提前。
- 测试 seam：`BootProgress` 注入 `impl Write` 与 tick 间隔（不 sleep 真实 150ms）；CLI 层用 `ensure_host_with` 假 spawn 断言清行后诊断仍在。
- 与 c3040 无 `depends_on`：进度只包 ensure future；c3040 落地后替换等待自动被覆盖。
- 任务拆分见 [tasks.md](./tasks.md)。

## 测试

- 单测：`BootProgress` 假 writer；150ms 前无输出；帧序列；finish 清行；error 清行；非 TTY 不输出。
- 单测：`with_boot_progress` 快完成不输出；慢完成输出。
- CLI 预检：用 `ensure_host_with` + wiremock / fake spawn 覆盖 Ready、超时、失败；断言进度清行后诊断仍在。
- BDD：`cli-entry` r1391 / `user-experience` r3065 新场景。
- 手测：无 Host 启动 TUI 看 spinner；已有 Host 不闪；kill Host / 占用端口看失败诊断；`--no-ensure-host` 无 spinner。
- 门禁：`just fmt`、`just test`、相关 clippy、`just qa`。

## 风险

- Windows 旧终端对 ANSI 清行支持差异；产品 TUI 已用 crossterm，沿用其能力或做 fallback。
- spinner 与 trust gate / stderr 诊断交错；finish 必须在 trust gate 前。
- 30s 超时期间 spinner 持续；不额外造取消语义。
- 非 TTY 误输出；必须 `stderr.is_terminal()` 门禁。
- 进度面被误解为产品画布；文案只用 `Starting host`，不画 header / 假 session。
