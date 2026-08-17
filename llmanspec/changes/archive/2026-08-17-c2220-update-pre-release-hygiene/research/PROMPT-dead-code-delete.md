# 派工 prompt：按 triage 立刻删无争议死码（c2220 执行切片）

把下面整段交给执行 agent。只删表里「可立刻删」清单；**拿不准 5 项一律不动**。

---

## 难度

低～中。机械删除 + 去 `allow`，但每条必须 `rg` 复核仍为零引用，删完要编译绿。不要发明新抽象。

## 在哪开工

**推荐独立 worktree + 分支**（主仓可能同时在改 TUI PreviewInject，会抢 `scrollback.rs` / `keybindings.rs`）：

```bash
wt switch --create --no-cd -y c2220-dead-delete
cd ../xylitol.c2220-dead-delete   # 或 wt 实际路径
eval "$(just cargo-wt-env)"       # 禁止共用 CARGO_TARGET_DIR
```

若确认主仓没有并行改同一文件，可在当前仓开分支 `c2220-dead-delete`，**不要直接 commit 到 main**。

## 读

1. `.claude/skills/audit-dead-code/SKILL.md`
2. 根 `AGENTS.md`「Pre-0.0.1 卫生」
3. `llmanspec/changes/c2220-update-pre-release-hygiene/research/dead-code-triage.md` → 「本 wt 可立刻删清单」+「拿不准」

以 **现时代码行号** 为准，表里的行号可能已漂。

## 做（仅这些）

按 triage §「可立刻删清单」1–7：

1. 删 `packages/xylitol-tui/examples/agent_demo_impl.rs` 的 `travel_path_with_replies`（确认 `path_ids_to` 仍被 Enter travel 使用）。
2. 删 `packages/xylitol-tui/src/terminal.rs` 四个仅文档化、字节已内联的常量：`KITTY_PUSH_SEQUENCE` / `KITTY_POP_QUERY_SEQUENCE` / `KITTY_POP_SEQUENCE` / `DA_QUERY_SEQUENCE`。协议字节留在 `KITTY_KEYBOARD_PROTOCOL_QUERY`。
3. 删 `src/infra/tools/accumulator.rs` 的 `OutputSnapshot::full_content` 与 `total_bytes`；简化 `finish()`；同步改同文件单测断言。
4. 删 `src/infra/clipboard/image.rs` 的 `base64_decode`；确认 clipboard 模块不因此空。
5. 删 `src/app/tui/keybindings.rs` 的 `install_product_keybindings`（零调用；demo/生产都不走它）。
6. **去 allow、不删符号**：triage 清单第 6 项列出的 `packages/xylitol-tui/tests/support/**` 与 `scrollback.rs` `clear_misses`。去 allow 后若 clippy/rustc 报 dead_code → **停下来写进 PR 说明，不要把 allow 加回去当完成**，也不要擅自扩删到拿不准项。
7. 去 `terminal.rs` `MODIFY_OTHER_KEYS_ENABLE` / `_DISABLE` 上过时的 `allow(dead_code)`（已被 rearm/stop 调用）。

每删一条：`rg` 符号名确认无残留引用（含测试）。

## 禁止

- 拿不准节 5 项：`TruncationResult` 死字段、bash `cfg(test)` seam、server lock RAII 字段、`ResolvedProfile`、trace `request_id`
- `XyRemoteDriver` 及 remote.rs 预留
- 改 live specs、playground、词表
- 新 `#[allow(dead_code)]`
- 兼容 shim

## 验证

```bash
eval "$(just cargo-wt-env)"   # 若在 wt
cargo build --all-features
cargo test --lib
# 若动了 xylitol-tui：
just test-tui
```

提交前至少 `just lint` + 相关测。能跑 `just qa quiet` 更好。

## 提交

分支 `c2220-dead-delete`。标题：`chore: delete unpublished dead code from c2220 triage`。

PR/commit 说明逐条对照清单（删了什么 / 去了哪条 allow）。不要 ff 进 main；等 review。
