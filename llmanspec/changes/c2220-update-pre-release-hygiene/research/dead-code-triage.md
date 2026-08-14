# 死代码分诊表（c2220）

> 扫描时间：2026-08-14（worktree `c2220-dead-code`）。扫描对象：`rg -n 'allow(dead_code)' src packages --type rust` 全部命中（含 `cfg_attr(test, allow(dead_code))` 变体，共 49 处）。
> 判定规则见 skill `audit-dead-code`：**真死**（编译期零引用）/ **逻辑死**（有引用但无真实入口触达）/ **预留**（架构意图明确 + 近期落地计划）。
> 「入口链」= `main.rs → lib::run → app::cli::run` 的实际分支（print / `--rpc` / tui / 子命令）。

## 已知热点复核（skill 旧表 vs 现时代码）

| skill 旧表条目 | 现时代码事实 | 结论 |
|---|---|---|
| `app/gui.rs` + `gui` feature | 已不存在：`src/app/` 无 gui 模块，`Cargo.toml` 无 `gui` feature | 旧表过时，无此条目 |
| `app/rpc.rs::Command` 全集 | `Command` 现为 `src/protocol/wire/command.rs`（24 变体），经 `dispatch.rs` 由 server REST + tui slash 双面消费，且不再有 `allow(dead_code)` | 旧表过时；该热点已激活 |
| `XyRemoteDriver` | 仍存在（`app/core/driver/remote.rs`），全仓库零实例化，仅 doc 引用 | 预留成立，见下表 remote.rs 三行 |
| `terminal.rs` allow | 仍存在：7 处（4 常量文档化 + modifyOtherKeys 预留） | 见下表 #11-17 |

## 全量分诊表

### packages/xylitol-tui（库）

| 路径 | 符号 | 判定 | 建议 | 依据（入口链或零引用） |
|---|---|---|---|---|
| src/terminal.rs:32 | `KITTY_PUSH_SEQUENCE` const | 逻辑死 | **可删**（常量直接内联进 `KITTY_KEYBOARD_PROTOCOL_QUERY`，删除零行为变化；协议字节保留在同文件常量里） | `rg` 零引用；仅「documented for clarity」注释为唯一存在理由；本仓无外部 SemVer 客户，符合 c2220 规则 5 |
| src/terminal.rs:36 | `KITTY_POP_QUERY_SEQUENCE` const | 逻辑死 | **可删**（同上，字节串已内联于 `KITTY_KEYBOARD_PROTOCOL_QUERY`） | 零引用 |
| src/terminal.rs:39 | `KITTY_POP_SEQUENCE` const | 逻辑死 | **可删**（同上；teardown 只用 `KITTY_POP_ALL_SEQUENCE`） | 零引用 |
| src/terminal.rs:45 | `DA_QUERY_SEQUENCE` const | 逻辑死 | **可删**（同上，已内联） | 零引用 |
| src/terminal.rs:55 | `MODIFY_OTHER_KEYS_ENABLE` / `_DISABLE` const 对 | 逻辑死→已被使用 | **去 allow**（两个常量都被真实调用：`_ENABLE` 在 `rearm_keyboard_after_alt_screen`（:261）实际写 stdout，`_DISABLE` 在 `disable_modify_other_keys`（:284，被 `stop`/`leave_alternate_screen` 调用）；无 cfg 门控） | 经 `enter_alternate_screen`/`stop` 真实路径触达 |
| src/terminal.rs:234 | `CrosstermTerminal::enable_modify_other_keys` | 预留 | 留（route-B 协商入口，同文件 :235-240 有注释与落地条件；当前与 rearm 内联语义重复，`_ENABLE`/`_DISABLE` 是私有方法时只有 `_ENABLE` 可能残留 unused——见行内） | 有注释+落地条件，符合预留形态 |
| src/utils.rs:378 | `AnsiCodeTracker::clear` | 预留 | 留（已有「预留：对齐 pi AnsiCodeTracker.clear」注释；`reset`+hyperlink 清除确实被 extract_segments 完整移植需要） | 有注释 |
| src/utils.rs:431 | `AnsiCodeTracker::has_active_codes` | 预留 | 留（同上注释） | 有注释 |
| examples/agent_demo_impl.rs:588 | `travel_path_with_replies` | 真死 | **可删**（示例内零调用；`path_ids_to` 被 Enter travel 真实使用，本函数被其语义取代） | 示例文件内 `rg` 零调用 |

### packages/xylitol-tui/tests/support（测试 harness）

| 路径 | 符号 | 判定 | 建议 | 依据 |
|---|---|---|---|---|
| support/vt_feed.rs:149 | `feed_vt` | 逻辑死→已被使用 | **去 allow**（可立刻执行，纯减负） | 5 个 test target 均引用：input/property/virtual_terminal/tui_integration/interaction 测试 |
| support/mod.rs:69 | `impl VirtualTerminal` | 逻辑死→实际使用 | **去 allow**（`viewport`/`cell`/`cursor_position`/`grid_*` 被各 target 使用） | 多 target 交叉使用 |
| support/mod.rs:93 | `VirtualTerminal::resize` | 逻辑死→被使用 | **去 allow** | 被 trait `Terminal::set_size_hint` 转发调用，且多 target 驱动 resize |
| support/mod.rs:148 | `viewport_cell` | 逻辑死→被使用 | **去 allow** | agent_demo_test(:1592,:1683,:1708) / interaction_modes_test(:1041) 真实调用 |
| support/mod.rs:178 | `title` | 真死（无调用） | 留 | 各 target 未用，但属 harness API 完整性（OSC 标题断言备用） |
| support/mod.rs:658 | `impl LoggingVirtualTerminal` | 逻辑死→实际使用 | **去 allow** | `clear_writes`/`count_occurrences` 被 virtual_terminal_test 大量使用 |
| support/mod.rs:677 | `raw_writes` | 真死（无调用） | 留 | harness API；注释说明未来差分渲染断言需要（pi `clearWrites` 模式） |
| support/mod.rs:727 | `inner` | 真死（无调用） | 留 | 经 `Deref` 访问；显式 inner 用于需 `&VirtualTerminal` 的测试（API 完整性） |
| support/mod.rs:874 | `impl Component for MutableComponent` | 逻辑死→被使用 | **去 allow** | virtual_terminal_test 7 处直接构造 + `mount_shared`（harness_test 2 处） |
| support/mod.rs:894 | `impl TuiTestHarness` | 逻辑死→实际使用 | **去 allow** | 多 target 使用其方法 |
| support/mod.rs:911 | `mount_shared` | 逻辑死→被使用 | **去 allow** | harness_test :22/:78 两处调用 |
| support/mod.rs:963 | `assert_cursor_at` | 真死（无调用） | 留 | 注释「used by later editor-port tests」（未来 editor-port 变更激活） |
| support/mod.rs:975 | `assert_cell_text` | 真死（无调用） | 留 | 同上（未来 editor-port 变更激活） |
| support/mod.rs:990 | `viewport_snapshot` | 逻辑死→被使用 | **去 allow** | snapshot_test :25 调用 |
| support/mod.rs:1031 | `render_row_annotated` | 逻辑死→被使用 | **去 allow** | 被 viewport_snapshot 调用（同一 test 模块） |
| support/mod.rs:1056 | `style_tag` | 逻辑死→被使用 | **去 allow** | 被 render_row_annotated 调用（同一 test 模块） |

### src（主 crate）

| 路径 | 符号 | 判定 | 建议 | 依据（入口链或零引用） |
|---|---|---|---|---|
| protocol/model/config.rs:85 | `ResolvedProfile` struct | 预留 | 留（加注释「为 c1xxx profile 能力预留；当前仅 system_prompt 字段被消费」） | 构造于 `AppConfig::resolve_profile/resolve_default_profile`，被 `bootstrap.rs:495`、`in_process.rs:1447` 消费 `system_prompt`；其余字段未用但这是 profile 数据契约的壳 |
| infra/tools/truncate.rs:15 | `TruncationResult` struct | 逻辑死 | 留（真死字段 `truncated_by`/`output_lines`/`output_bytes`/`max_lines`/`max_bytes` 可删） | 被 read/find/grep 工具用 `truncate_head` 构造；`truncate_head` 仅 `content`/`truncated`/`total_lines`/`total_bytes` 字段被读 |
| infra/tools/truncate.rs:198 | `TruncatedLine::was_truncated` | 逻辑死 | 留（仅单测读；这是「发现/报告」语义，保留在库级 API 中） | `truncate_line` 返回给 grep；`was_truncated` 仅 `truncate.rs` 单测读 |
| infra/tools/accumulator.rs:205 | `OutputSnapshot::full_content` | 真死 | **可删**（构造函数有代价：`finish` 每次全量组装 `full_content`，即使无调用方；删后连带简化 `finish`） | 全仓库零读；bash 工具只用 `display_content`/`truncated`；单测 `accumulator.rs:289-300` 读的是构造时的 `full_content`（删字段需同步删这两处断言） |
| infra/tools/accumulator.rs:208 | `OutputSnapshot::total_bytes` | 真死 | **可删**（同上连带） | 全仓库零读 |
| infra/tools/bash.rs:32 | `BashArgs::description` | 预留 | 留（schema 给 LLM 的 UX 字段；删了会改 tool schema。改注释：不再「未用」，而是「schema 占位」） | `TypedTool::parameters_schema` 经 serde 生成；executor 不读 |
| infra/tools/bash.rs:45 | `BashOperations` trait | 预留（test-only seam） | 留（MockBash 测试 seam 是真实测试需求；注释已说明。可考虑改为 `#[cfg(test)]` 以强约束） | 仅 `bash.rs` 单测 `MockBash` 实现；生产仅 `RealBashOperations` |
| infra/tools/bash.rs:246 | `BashTool::with_operations` | 预留（test-only seam） | 留（同 45；建议 `#[cfg(test)]`） | 仅 `test_bash_operations_trait_mock` 使用 |
| infra/tools/bash.rs:253 | `BashTool::with_hooks` | 预留（test-only seam） | 留（同 45；建议 `#[cfg(test)]`） | 仅 `test_bash_hooks_get_called` 使用 |
| app/tui/widgets/scrollback.rs:606 | `ScrollbackPaintCache::clear_misses` | 逻辑死→被使用 | **去 allow**（`#[cfg(test)]` 下被 `UiRoot::clear_scrollback_entry_misses_for_test`（`layout/root/mod.rs:1122`）调用，无需 allow） | `#[cfg(test)]` 测试 helper 链 |
| packages/xylitol-ai-bridge/src/provider/trace.rs:262 | `ProviderRequestTrace::request_id` | 预留 | 留（request_id 经 `Span::with_properties` 挂在 span 属性上输出到 trace；字段本身仅生命周期持有。改注释说明其经 span 属性消费） | fastrace span 属性写入 `request_id`；字段本身无读但 span 已持有 |
| infra/clipboard/image.rs:243 | `base64_decode` | 真死 | **可删**（镜像 osc52.rs 的编码器；无解码需求。删时检查 `mod clipboard` 是否因此变空） | 全仓库零引用；仅同文件 `base_image_mime` 等使用；「mirrors the encoder in osc52.rs」注释 |
| infra/resource/loader.rs:94 | `DefaultResourceLoader::reload` | 预留（port 实现） | 留（`XyReloadable` port 的实现；c1120 意图由 `XyReloadable` 契约驱动。加注释「port 契约；inherent 方法为生产 reload 路径」） | `impl XyReloadable` 转发；生产 `/reload` 走 `bootstrap::reload_skills` + `reload_prompt_context` 新建 loader |
| infra/resource/loader.rs:497 | `impl XyReloadable for DefaultResourceLoader` | 预留（port 实现） | 留（port 契约不构成死码；`reload_runtime` 经 `XyReloadable::reload` 在单测驱动） | `c1120` 文档指明「product `/reload` 将 orchestrate concrete reloads」；单测 `test_reload_picks_up_changed_agents_md` 用 `XyReloadable::reload` |
| app/tui/keybindings.rs:266 | `install_product_keybindings` | 真死 | **可删**（「demo / intentional GLOBAL install」注释过时——demo 用 `set_keybindings` 直接构建，生产 HostSession 走 Scope；`ensure_product_catalog` 只用 `install_product_keybindings_defaults_only`） | 全仓库零调用 |
| app/server/runtime.rs:26 | `RunningServer::lock` | 逻辑死 | 留（生命周期持有：`ServerLock` 文件存在依赖此字段存活；Drop 才删锁文件。改注释为「held for lifetime so lock file persists」） | 锁文件靠字段持有；`acquire_lock_and_bind` 返回 `ServerLock` |
| app/server/lock.rs:59 | `ServerLock::handle` | 逻辑死 | 留（同上：文件句柄保持打开是锁的机制，不是数据读取） | 同 |
| app/core/dispatch.rs:43 | `DispatchOutcome` enum | 逻辑死 | 留（server REST + tui slash 双面 feature-gated 读取；default feature 编译时未读字段报 dead_code。保持 enum 级 allow，见文件内注释） | `map_dispatch`/`slash.rs` 均匹配其变体 |
| app/core/driver/remote.rs:30 | `XyRemoteDriver` struct | 预留 | 留（注释已写明：远程薄端接线后由该面实例化；落地条件：远程客户端应用面开闸。`docs/roadmaps/Cloud-Agent与Web控制台.md` 是近期方向） | 全仓库零实例化；仅 doc 引用 |
| app/core/driver/remote.rs:43 | `impl XyRemoteDriver` | 预留 | 留（同 struct，注释已写明） | 同上 |
| app/core/driver/remote.rs:820 | `urlencoding_loose` | 逻辑死 | 留（仅被预留的 `XyRemoteDriver::set_model` 调用；与 struct 同生命周期，见注释） | `set_model` 调用；struct 预留 |
| app/tui/effects/mod.rs:63 | `kick_footer_token_refresh` | 逻辑死 | 留（生产 `drain_pending` 路径使用；`#[cfg_attr(test, allow(dead_code))]` 精确到 test 编译。这个 allow 是必要的——不是死码压制，而是 test cfg 下的真实差异） | `pending_ui.rs:98` 生产调用 |

## 本 wt 可立刻删清单（无争议真死，≤15 项）

按优先级：

1. **`packages/xylitol-tui/examples/agent_demo_impl.rs:588` `travel_path_with_replies`** —— 零调用，语义被 `path_ids_to` + Enter travel 取代。
2. **`packages/xylitol-tui/src/terminal.rs:32/36/39/45` 4 个内联常量** —— `KITTY_PUSH_SEQUENCE` / `KITTY_POP_QUERY_SEQUENCE` / `KITTY_POP_SEQUENCE` / `DA_QUERY_SEQUENCE`，协议字节串已内联于 `KITTY_KEYBOARD_PROTOCOL_QUERY`（:50）；本包无 feature 门控（`[features]` 仅 `highlight`），`rg` 零引用，删零行为变化。
3. **`src/infra/tools/accumulator.rs` `OutputSnapshot::full_content` + `total_bytes` 字段** —— 全仓库零读；删后同步简化 `finish()` 组装逻辑与单测两处断言。
4. **`src/infra/clipboard/image.rs` `base64_decode`** —— **Linux `rg` 盲区，不可删。** 唯一生产调用者是 `#[cfg(target_os = "windows")]` 的 `read_windows_clipboard_image`。执行切片改为去 allow + 更名 `decode_base64` + `#[cfg(any(target_os = "windows", test))]`。以后扫死码 MUST 带 `--cfg` / 读 `cfg(target_os)` 调用链，不能只信 Linux 零引用。
5. **`src/app/tui/keybindings.rs:266` `install_product_keybindings`** —— 全仓库零调用；「demo GLOBAL install」注释过时（demo 用 `set_keybindings`，生产走 Scope）。
6. **去 allow（非删代码）**：
   - **`scrollback.rs` `clear_misses`**：✅ 已去 allow（`#[cfg(test)]`，经 `UiRoot` test helper 触达）。
   - **`packages/xylitol-tui/tests/support/**` 其余 11 处：❌ 不可去 allow。**判定被证伪**——12 个 test target 各自独立 `mod support`，去 allow 后约 30 个新 `dead_code`（基线仅 `agent_demo_test` 局部 5 个）。这些 allow 是跨 target 必需的 harness API 完整性，不是「已被使用所以 allow 多余」。重新分诊前保持原 allow。

7. **`packages/xylitol-tui/src/terminal.rs:55` `MODIFY_OTHER_KEYS_ENABLE`/`_DISABLE`** —— 常量已被真实调用（rearm :261 写 `_ENABLE`、`disable_modify_other_keys` :284 写 `_DISABLE`，后者被 `stop`/`leave_alternate_screen` 调用），allow 已过时，去 allow 零风险。

## 拿不准 / 需变更决策（单独一节）

1. **`infra/tools/truncate.rs:15` `TruncationResult` 的 5 个真死字段**（`truncated_by`/`output_lines`/`output_bytes`/`max_lines`/`max_bytes`）：全仓库仅单测读。删除会让 `truncate_head` 返回值变薄，但改动跨 read/find/grep 三工具 + 单测。属于「可删但影响面略大」——建议留到下个 tool 相关变更一并做。
2. **`infra/tools/bash.rs:45/246/253` test-only seam**：当前 allow 是「生产编译压死码」。建议改为 `#[cfg(test)]` 强约束（非删代码），但会连带 `BashOperations` trait 的 pub 暴露语义。需确认 MockBash 是否未来有 harness 复用场景。
3. **`app/server/runtime.rs:26 lock` / `app/server/lock.rs:59 handle`**：判定为「生命周期持有」非死码；但严格说这是 RAII 惯用法，allow 是压制「永不读」字段。可保留 allow，但建议注释从「Never read directly」改为「held for lock lifetime」。
4. **`protocol/model/config.rs:85 ResolvedProfile`**：profile 能力在 c1xxx 有近期落地（`bootstrap.rs` 已在消费 `system_prompt`）；保留但需补「为 profile 能力预留」注释。
5. **`packages/xylitol-ai-bridge/src/provider/trace.rs:262 request_id`**：字段本身无读，但 `request_id` 经 fastrace span 属性进入 provider-trace JSONL。这是「数据经反射/序列化消费」的灰色地带，建议保留并加注释说明消费路径。

## 与 pre-release-hygiene.md 的交叉

- `pre-release-hygiene.md` 提到 `app/core/driver/remote.rs` 的 allow「须分诊，不是一律删（remote driver 可能是嵌入/server 预留）」—— 本表确认：**预留成立**，注释已含落地条件（远程薄端接线后实例化），与 `docs/roadmaps/Cloud-Agent与Web控制台.md` 方向一致。
- `pre-release-hygiene.md` 提到 `infra bash/truncate` —— 本表确认 bash 是 test-only seam（建议 cfg(test) 强约束），truncate 是真死字段（拿不准节 1）。
- `pre-release-hygiene.md` 提到「测试 support 等」—— 本表初判 tests/support 可立刻去 allow；**c2220-dead-delete 执行证伪**（跨 target 独立 `mod support`）。仅 `clear_misses` 去 allow 成立。

## 执行结果（2026-08-14，分支 `c2220-dead-delete` / `3e36b184`）

清单 1–3、5、7 与 `clear_misses` 已删/去 allow。第 4 项未删（Windows cfg）。第 6 项 support 去 allow 已 revert。拿不准 5 项与 `XyRemoteDriver` 未动。

## 执行注意事项

- 删除任何符号后必须跑 `cargo build --all-features` + `just test`，清 `unused import`。
- 去 allow 后若编译器报新 dead_code，说明该符号确实无真实使用 → 回到上表分诊。
- `rg` 零引用不足以判死：MUST 检查 `#[cfg(target_os = …)]` 与「每 test target 一份 `mod support`」这类分编译单元。
