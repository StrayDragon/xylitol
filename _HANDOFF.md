# _HANDOFF — 会话交接（2026-07-03）

> 分支：`feat/tui-dev`　|　基准：`main`　|　工作树：干净
> 本文件是临时交接文档，下一次会话可据此继续；落地后可删。

## 一、本轮完成了什么

### 1. c336-unify-app-core-shared-layer（已实现 + 归档）

三面共享层落地，三个关键决策（都基于代码事实）：

- **共享装配 `app::core::bootstrap`**：config→registry→trust→resource→build_agent 全链路，print/tui/server 三面共用。server 补回了 context_files/append_system_prompt 发现（移除 headless 跳过）。
- **共享分发 `app::core::dispatch`** + Driver trait 全量扩展：dispatch 是纯 Command→Driver 分派器；tui `/model` 经此实际切换（移除 stub）。InProcessDriver 实现全套，RemoteDriver 占位（命令路由待 server 补）。
- **移除 rpc 模式**：调查发现 rpc 零外部消费者 + 零真实测试覆盖 + 784 行违反 spec ip4（「该是薄 stdio 传输」）。移除而非重写（remote 用例由 server WS/REST 承担）。

相关 commit：`9712e7a`(propose) → `68af84d`(实现) → `5621461`(归档) → `b8ef8a9`(AGENTS.md 同步)。

### 2. server AGENTS.md 注入验证（用 log 基础设施）

`XYLITOL_DEBUG=1` 实测：`caller=server trusted=true context_files=1`，AGENTS.md（122 bytes）被 bootstrap 发现。c336 的核心行为变更端到端证实。

附带给 bootstrap 加了结构化 trace 点（caller/trusted/context_files）—— commit `60cb079`。

> **小知识**：server 在**未信任**目录下不发现 AGENTS.md（spec t5，正确行为）。要验证 trusted 分支需先 `set_trust` 或 `--trust`。server subcommand 当前没 `--trust` flag——这是个已知小缺口（信任决策本应在首次交互时做），非 bug。

### 3. clipboard 卡死修复（预存 bug，与 c336 无关）

`cargo test --lib` 卡住的根因：`spawn_detached` spawn 的 wl-copy daemonize，其后台进程继承 cargo test harness 的 stdout pipe fd，harness 等不到 EOF → 死锁。

修复（两 commit）：
- `2d23de2`：`spawn_detached` 的 `wait()` 改 `try_wait` 轮询 500ms（生产路径改进）
- `899c06b`：测试标 `#[ignore]`（根因是 fd 继承，单元测试不该 spawn 会 daemonize 的真实系统命令）

现在 `cargo test --lib` 30s 完成，不再卡。

### 4. 三个 TUI 组件 draft（specified 阶段，约束已订，未实现）

基于 codex/pi/kimi-code 三家实际代码调研（subagent 读取），拆成三个独立 draft：

| draft | 约束 | 核心决策 |
|---|---|---|
| **c355** tui-markdown-render | tui60/tui61 | user/assistant **共用** MarkdownRenderer（pi 模式），pulldown-cmark；MVP 不做 syntect/表格/流式增量 |
| **c356** tui-list-selection | tui62/tui63 | ListSelection **纯逻辑状态机**（kimi-code SearchableList 模式）+ ApprovalOverlay **组合**它；审批经 Driver 回写 |
| **c357** tui-command-popup | tui64 | `/` 触发，消费 `Driver::get_commands()`，组合 ListSelection，键位互斥 |

三个都是 `specified` 阶段（有 proposal + 约束 spec.toon，**无 tasks/design**）。commit `c05b9e3`。

## 二、当前状态

### active changes
```
c355-tui-markdown-render  specified   no tasks
c356-tui-list-selection   specified   no tasks
c357-tui-command-popup    specified   no tasks
```

### 校验状态（全部 clean）
- `cargo test --lib`：479 passed + 2 ignored，30s（不再卡）
- `cargo test --test bdd`：85 passed
- `cargo test --lib arch_guard`：4 passed
- `cargo clippy`（lib+bins）：零 warning
- `llman sdd validate --all`：46 passed

### 未推送
本轮所有 commit（c336 实现/归档 + clipboard 修复 + trace + 三个 draft）都在 `feat/tui-dev` 本地，**尚未 push**。

## 三、下一步选项（按建议优先级）

### 选项 A：full 化并实现 c355（markdown 渲染）—— 推荐
理由：c355 是 c356/c357 的渲染基础（审批浮层的「查看详情」、命令面板的描述都要渲染 markdown）。
- 用 `/llman-sdd-apply c355-tui-markdown-render`（但先要 `/llman-sdd-continue` 补 design.md + tasks.md 长大到 full）
- 关键实现点：pulldown-cmark 依赖、MarkdownStyle、user/assistant 共用、finalize 后全量渲染（流式留后续）
- 风险点：RenderedLine 从单行变多行，需回归 CJK 换行（tui41）

### 选项 B：实现 c360（TUI remote 模式）
激活 RemoteDriver（传输层已就绪）：CLI 加 `--remote <url>` → 构造 RemoteDriver → 喂给 `tui::run`。这是用户「想法 2」（TUI 可选 server 交互模式）的落地。
- spec tui2 只禁止「默认 RemoteDriver」，加 `--remote` 完全合规
- 铺路价值：分离进程架构；验证 server 作可观测后端；为后续 web 客户端探路

### 选项 C：先放着 draft，做别的
三个 draft 保持 specified 状态，回头按需 full 化。

## 四、关键代码事实（供后续 full 化参考）

### c336 留下的基础设施（c355-c357 会用到）
- **dispatch 全套 Command 分派**已就绪（`app::core::dispatch`），tui 加新 slash 命令只需在 `tui/commands.rs` 加解析 + 调 `shared_dispatch`
- **Driver trait 全套方法**已扩展（`app::core::driver`）：select_model/cycle_model/compact/export_*/get_messages/get_commands 等。c357 的 CommandPopup 直接消费 `Driver::get_commands()`
- **审批回写缺口**：c356 的 ApprovalOverlay 需 Driver 加 `approve_tool(call_id, approved)` 方法（当前 Driver trait 没有，ApproveTool 是 WS 专属被 dispatch 拒绝）。full 化 c356 时要补这个方法 + InProcessDriver 实现

### RenderedLine 现状（c355 要改的点）
```
src/app/tui/render.rs:55  pub enum RenderedLine { UserInput(String), AssistantText(String), ... }
src/app/tui/render.rs:79  RenderedLine::UserInput → Line::styled("❯ {prompt}", user_prompt())
src/app/tui/render.rs:80  RenderedLine::AssistantText → Line::styled(text, assistant())
```
当前都是单行 `Line`，无 markdown 解析。c355 要让 UserInput/AssistantText 经 MarkdownRenderer 产出**多行**。

### 三家调研结论（已写进各 proposal，这里是要点）
- **markdown**：抄 pi（user/assistant 共用 + MarkdownStyle 注入），用 codex 的 pulldown-cmark（Rust 原生），流式先跳过（pi/codex 的流式都复杂），表格/高亮先跳过
- **列表**：抄 kimi-code 的 SearchableList 纯状态机（Rust trait 自然，最薄），审批浮层组合它（codex 模式）
- **命令面板**：抄 codex 的 composer 内嵌下拉 + filter，组合 ListSelection

## 五、commit 链（本轮）
```
c05b9e3  docs(sdd): draft c355/c356/c357 — TUI 组件约束
60cb079  feat(core): trace resource discovery in bootstrap
899c06b  fix(clipboard): ignore test spawning daemonizing wl-copy
2d23de2  fix(clipboard): stop hanging on wl-copy (try_wait 轮询)
b8ef8a9  docs(agents): sync AGENTS.md with rpc removal + c336
5621461  docs(sdd): archive c336 (complete)
68af84d  refactor(core): unify assembly+dispatch, remove rpc mode (c336)
9712e7a  docs(sdd): propose c336, drop stale c330/c335/c345/c350
```
