# c405 Tasks — 五层 TUI 测试 harness

> 每块 ≤2h。校验命令在末尾。引用的 spec req：tt01-tt06（tui-testing）、r8（test-infra）。

## 阶段 0：依赖与目录骨架（0.5h）

- [x] 0.1 `packages/xylitol-tui/Cargo.toml` dev-dependencies 加 `insta = { version = "1", features = ["filters"] }`
- [x] 0.2 workspace 顶层 `Cargo.toml` dev-dependencies 加 `portable-pty = "0.9"`
- [x] 0.3 建 `tests/tui_e2e/` 目录（workspace 集成测试），加 `mod.rs` 占位
- [x] 0.4 `cargo check` 通过

## 阶段 1：键序列 helper（第 1 层，spec tt02）—— 通用化 MutableComponent（1.5h）

- [x] 1.1 读 `packages/xylitol-tui/tests/virtual_terminal_test.rs:178-184` 的 workaround 注释 + 局部 `MutableComponent` 定义，理解当前限制
- [x] 1.2 在 `tests/support/mod.rs` 加通用 `MutableComponent`（Rc<RefCell<Vec<String>>> 注入为 Component）
- [x] 1.3 加 `TuiTestHarness` 结构：封装 `TUI<LoggingVirtualTerminal>` + 提供 `mount` / `mount_shared` / `keys` / `focus` / `render`
- [x] 1.4 加便捷断言：`assert_text_contains` / `assert_cursor_at` / `assert_cell_text`
- [x] 1.5 重写 `tui_dispatch_input_reaches_focused_component`（virtual_terminal_test.rs:188）用新 harness，验证不再需要手工拼
- [x] 1.6 写新测试 `keys_helper_drives_multi_step_sequence`（harness_test.rs，对应 scenario tt02）
- [x] 1.7 删掉 virtual_terminal_test.rs 里旧的局部 `MutableComponent`，改用通用版
- [x] 1.8 `cargo test -p xylitol-tui` 通过（186 passed）

## 阶段 2：insta snapshot（第 2 层，spec tt03）—— 整屏 golden（1.5h）

- [x] 2.1 在 `tests/support/mod.rs` 加 `viewport_snapshot(harness)` 函数：把 cell grid 渲染成可读多行文本（含 SGR 标注 `[bold]text[/]`）
- [x] 2.2 写 `panel_renders_bordered_layout` snapshot（对应 scenario tt03）
- [x] 2.3 写 `select_list_renders_highlighted_first_item` snapshot（SelectList 第一项 reverse 高亮）
- [x] 2.4 写 `text_wraps_at_terminal_width` snapshot（15 列换行边界）
- [x] 2.5 接受全部新 snapshot（INSTA_UPDATE=always，3 个 .snap 已生成）
- [x] 2.6 `cargo test -p xylitol-tui --test snapshot_test` 通过（3 passed）

## 阶段 3：时序基建（第 3 层，spec tt04）—— Clock trait + paused time（1.5h）

> 注：本阶段只建抽象 + 骨架测试。paste-burst/autocomplete 的真实时序测试在后续移植变更（c410+）随功能一起写。

- [x] 3.1 在 `packages/xylitol-tui/src/` 加 `clock.rs`：`trait Clock { fn now(&self) -> Instant }` + `SystemClock` + `MockClock`
- [x] 3.2 `lib.rs` 导出 `Clock` / `SystemClock` / `MockClock`
- [x] 3.3 写 `mock_clock_advances_deterministically` + `mock_clock_window_boundary_exact`（对应 scenario tt04 前提）
- [x] 3.4 写 `debounce_fires_after_window_under_paused_time`（tokio `start_paused` 骨架）
- [x] 3.5 `cargo test -p xylitol-tui --lib clock` 通过（3 passed）

## 阶段 4：proptest 骨架（第 4 层，spec tt01）—— 状态机框架（1h）

> 注：本阶段只建框架 + 一个示例不变量。真实 editor 不变量在 editor 移植完成后补。

- [x] 4.1 `packages/xylitol-tui/Cargo.toml` dev-dependencies 加 `proptest = "1"`（阶段 0 已加）
- [x] 4.2 在 `tests/` 加 `property_test.rs`：`key_seq()` 策略生成随机按键序列（a-z / Enter / Backspace / 方向键 / Home / End）
- [x] 4.3 写 `input_never_panics_on_random_keys`（256 case，对应 spec tt01）
- [x] 4.4 写 `input_render_always_non_empty` 不变量（render 总有 prompt 行 + value UTF-8 不变）
- [x] 4.5 `cargo test -p xylitol-tui --test property_test` 通过（2 tests × 256 cases）

## 阶段 5a：portable-pty E2E（第 5 层主，spec tt05）—— 真链路集成（2h）

- [x] 5a.1 在 `tests/tui_e2e.rs` 写 `PtySession` + `CapturedScreen`：spawn `cargo run --example demo -p xylitol-tui`（无 driver 依赖，验证 crate 渲染管线），背景线程读 PTY 字节
- [x] 5a.2 `send_keys` / `drain(settle)` / `screen(cols,rows)` / `wait_for(needle,timeout)` helper（读字节喂 CapturedScreen vte 解析）
- [x] 5a.3 写 `pty_demo_starts_and_renders`（等 "Quit" 出现，对应 scenario tt05 简化：证明 demo 在真 PTY 下渲染）
- [x] 5a.4 写 `pty_demo_survives_keypresses`（发按键后 demo 不崩）
- [x] 5a.5 标记所有 PtySession 测试 `#[ignore]`（spec r8：不进默认矩阵）
- [x] 5a.6 `cargo check --test tui_e2e` 通过（编译验证；实跑需本地 `cargo test --test tui_e2e -- --ignored`）

> 设计调整：spawn `xylitol-tui` demo example（非 `xylitol` 完整二进制）以解耦 LLM provider 依赖，专注验证 crate 渲染管线 + crossterm 真 PTY 行为。Ctrl+方向键等 crossterm 解析测试随 editor 移植（demo 当前无 editor）补。

## 阶段 5b：tmux 冒烟（第 5 层辅，spec tt06）—— 真终端（1.5h）

- [x] 5b.1 `TmuxSession` struct：`spawn_demo(cols,rows)` 调 `tmux new-session -d -s xyl_e2e_{pid}`，name 带 PID（spec r8）
- [x] 5b.2 `send(&[&str])` / `send_text(&str)` / `capture(with_sgr) -> String`
- [x] 5b.3 `wait_for(needle, timeout)` 轮询 capture（30ms 间隔，禁止固定 sleep）
- [x] 5b.4 `Drop` for `TmuxSession`：`tmux kill-session -t <name>`（spec r8：panic 也不残留）
- [x] 5b.5 `require_tmux!()` 宏：检查 tmux 存在否则 `return` 跳过
- [x] 5b.6 `tmux_demo_starts_and_shows_content`（wait_for "Quit"，对应 scenario tt06）
- [x] 5b.7 `tmux_captures_styled_output`（capture(true) 含 SGR `\x1b[` 序列，对应 scenario tt06）
- [x] 5b.8 全部 `#[ignore]`；`TERM=xterm-256color` 在 session 创建时设
- [x] 5b.9 `cargo check --test tui_e2e` 通过（编译验证；实跑需 tmux + `cargo test --test tui_e2e -- --ignored`）

## 阶段 6：justfile + 文档（0.5h）

- [x] 6.1 justfile 加 `test-tui-e2e` target：`cargo test --test tui_e2e -- --ignored`
- [x] 6.2 justfile 加 `test-tui-e2e-pty` / `test-tui-e2e-tmux` 两个细分 target
- [x] 6.3 更新 `_HANDOFF.md`：新增「五、TUI 测试 harness（c405）」节，记录五层架构落点与约定
- [x] 6.4 新建 `packages/xylitol-tui/AGENTS.md`：记录测试约定（第 1-4 层落点 + 新增测试规则）

## 阶段 7：全量校验（0.5h）

- [x] 7.1 `cargo test -p xylitol-tui` 全量通过（194 passed，含 snapshot 3 + harness 3 + clock 3 + property 2×256）
- [x] 7.2 `cargo test --workspace`（默认矩阵，不含 `--ignored`）通过（479 passed + 2 ignored E2E）
- [x] 7.3 `just qa` 核心通过（fmt OK + clippy `--all-targets -D warnings` clean + test 全绿）；doc-check + prek 为环境依赖
- [x] 7.4 `just test-tui-e2e` 实跑通过：2 pty + 2 tmux 全绿（tmux 3.7b，真 PTY）
- [x] 7.5 `llman sdd validate c405-add-tui-test-harness --strict` 通过（见下）

## 校验命令

```bash
cargo test -p xylitol-tui                    # 第 1-4 层
cargo test --workspace                       # 默认矩阵（不含 E2E）
cargo test --test tui_e2e -- --ignored       # 第 5 层（需要 tmux + 真 PTY）
just qa                                      # fmt + clippy + test + docs
llman sdd validate c405-add-tui-test-harness --strict
```
