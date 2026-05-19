# c110-update-tui-codex-parity Tasks

- [ ] **1 — 调研 codex-tui 关键交互语义**：基于 `../codex/codex-rs/tui/` 代码与 `tooltips.txt`，整理必须复刻的行为清单（按键→状态机→行为），并写入 `design.md`。
  - Evidence: `../codex/codex-rs/tui/src/keymap.rs`, `../codex/codex-rs/tui/src/bottom_pane/chat_composer.rs`, `../codex/codex-rs/tui/tooltips.txt`

- [ ] **2 — Keymap 抽象落地**：在 `src/interface/tui/` 新增 keymap 模块（KeyBinding/RuntimeKeymap + reserved keys），并把 `app.rs` 的全局快捷键改为 keymap dispatch。
  - Verify: `cargo test --features ui-tui`

- [ ] **3 — Composer 语义对齐（Tab/Enter/Esc）**：重写 `InputComponent` 为 codex 风格状态机：slash popup、bang shell、queue vs submit 规则、Esc 取消。
  - Verify: 新增最少单元测试覆盖 Tab queue + `!` 特例 + Esc cancel

- [ ] **4 — Backtrack（Esc Esc 编辑上一条）**：实现最小 backtrack：当 composer 为空时，连续 Esc 进入“编辑上一条 user message”模式；Enter 确认并回填 composer。
  - Verify: `cargo test --features ui-tui`

- [ ] **5 — Transcript overlay（Ctrl+T）**：加入全屏 transcript/pager overlay（仅浏览，不改 review UI），支持 q/Esc 退出与滚动。
  - Verify: 手动 + `cargo test --features ui-tui`

- [ ] **6 — Copy last response（Ctrl+O / /copy）**：实现复制最新 assistant 响应为 markdown（macOS: `pbcopy`；fallback: 写到临时文件并提示路径）。
  - Verify: `cargo test --features ui-tui`

- [ ] **7 — Raw output mode（Alt+R）**：实现 raw scrollback mode toggle（改变渲染策略/禁用 markdown 装饰以便终端复制）。
  - Verify: 手动 + `cargo test --features ui-tui`

- [ ] **8 — Spec delta**：补齐 `llmanspec/changes/c110-update-tui-codex-parity/specs/tui-interface/spec.md`：新增/修改 requirements + scenarios 覆盖 keymap/composer/backtrack/copy/raw/transcript。
  - Verify: `llman sdd validate c110-update-tui-codex-parity --strict --no-interactive`

- [ ] **9 — 回归验证**：`cargo fmt -- --check` + `cargo clippy` + `cargo test` + `cargo doc --no-deps --all-features`

- [ ] Archive: `llman sdd archive run c110-update-tui-codex-parity`
