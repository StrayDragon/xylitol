# 截断与通知文案 spec 合约全集（c3365 research）

查证日期：2026-11-18。方法：全量 grep `llmanspec/specs/`（截断/truncat/上限/limit/kept/dropped/文案串）+ 精读命中条款原文 + 交叉核对 crates/plugins/tests 实现与 BDD 步断言。路径均相对仓库根。未改 specs、未改 proposal、无 git 写操作。

## 1. 合约条款全集（按语义族）

### 1.1 工具输出截断（进会话历史，合约核心面）

| 条款 | 引用 | 钉死内容 |
|---|---|---|
| bash 输出截断 | `llmanspec/specs/infra-bash/infra-bash.feature:22-28`（@req:r1431） | 截断到配置 max bytes；完整输出溢出临时文件；返回**截断尾部**；**逐字**追加 pi 形脚注 `[Full output: <path>. Truncated: <N> lines shown (<limit> limit)]`，path 不可用时用 `(unavailable)`；**未截断不得追加**；边界不完整 UTF-8 须字符边界安全截断不 panic |
| 执行器 1MB 上限 | `llmanspec/specs/infra-bash/infra-bash.feature:68-74`（@req:r1428 场景） | stdout 超 1MB → 子进程被杀并返回截断输出 |
| bash 工具流式上限 | `llmanspec/specs/agent-tools/agent-tools.feature:124-138`（@req:r5） | 字节上限；超限杀子进程；场景断言截断详情、Full output 脚注、结果 JSON 无未截断全量 stdout |
| bash JSON 契约 | `llmanspec/specs/agent-tools/agent-tools.feature:467-474`（@req:r1147） | 截断时仅含截断视图（stdout/combined = display_content，含 Full output 脚注）+ truncated/full_output_path；未截断时完整输出且**无**脚注 |
| bash 累加器 | `llmanspec/specs/agent-tools/agent-tools.feature:440-465`（@req:r1139/r1140） | OutputAccumulator：尾部窗口 + 滚动阈值溢出临时文件 + 截断信息快照；r1140 写「配置 max_lines 100 与 max_bytes DEFAULT_MAX_BYTES」（**漂移观察**：实现无 max_lines 配置，见 §4） |
| UTF-8 安全截断 | `llmanspec/specs/agent-tools/agent-tools.feature:195-202`（@req:r8） | truncate_output 须字符边界感知切片，防 UTF-8 panic |
| read 截断 | `llmanspec/specs/agent-tools/agent-tools.feature:399-420`（@req:r1136） | **2000 行或 50KB** 处截断；offset 越界报告；截断时显示剩余行提示 |
| read 大文件拒绝 | `llmanspec/specs/agent-tools/agent-tools.feature:115-122`（@req:r4） | 超 MAX_FILE_SIZE（默认 10MB）**拒绝**（拒绝族，非截断） |
| ls/grep/find 条目上限 | `llmanspec/specs/agent-tools/agent-tools.feature:95-99`（@req:r42 场景）、`:322-341`（@req:r1158）、`:353-378`（@req:r1159） | limit 参数生效；超限「结果指示达到条目限制」（ls 场景）；grep/find 未钉文案与默认值 |
| edit diff 展示上限 | `llmanspec/specs/agent-tools/agent-tools.feature:313-321`（@req:r1157） | display_diff 展示行数硬上限，超限标注截断；模型正文不得嵌完整 diff |
| run_code 上限 | `llmanspec/specs/agent-codemode/agent-codemode.feature:46` | 输出须受 max_result_bytes 限制（选项钉死，默认值未钉）；details 须含截断标记（`llmanspec/specs/agent-tools/agent-tools.feature:649`） |
| 命令注入 4KB | `llmanspec/specs/agent-prompt/agent-prompt.feature:116` | 命令注入器以 4KB 截断（值钉死） |
| run_command_capture | `llmanspec/specs/infra-plugin/infra-plugin.feature:62` | 按字节上限截断（值未钉） |
| **MCP 结果截断** | `llmanspec/specs/infra-mcp/infra-mcp.feature:78-90`（@req:r2934） | 单次输出上限=**50 KiB**、与既有内置工具同一口径、**不得复用观测 IO 常量**；超限单段文本在进历史前截断并**标注保留与丢弃字节数及上限**，不得静默丢内容；多分片整体保留不得拍平；错误结果与非 MCP 工具不得截断；须 MCP 侧装配（有 servers 才注册）、可卸载、不得硬编码进适配器 |
| MCP 资源缓存 | `llmanspec/specs/infra-mcp/infra-mcp.feature:156-158`（@req:r3278） | LRU 64、**单条 256 KiB**、TTL 5 分钟（独立于 r2934 的另一上限面） |

### 1.2 观测面截断（不进会话历史，gated）

- `crates/xylitol-plugin-sdk/src/obs_gate.rs:8` `PROVIDER_TRACE_TEXT_MAX = 4096`；`:10` `OBSERVATION_IO_FULL_MAX = 65_536`；`:12-18` `truncate_observation_text` 按 **chars** 计数截断；`:42-48` `ObservationIoTier::max_chars()`（None=不截、Truncated=4096 chars、Full=65536 chars）。
- 应用点：`crates/xylitol-runtime/src/agent/runtime/obs.rs:49-55`（turn input preview 属性）、`:257-267`（tool.execute span input/output）——仅 tier 激活才附着。
- `llmanspec/specs/infra-otel/infra-otel.feature:81`、`:116`、`:124`：缺省 none **不得写入** observation I/O；truncated/full 且 span 闸激活才按档写入（同档硬顶截断）。`llmanspec/specs/infra-observability/infra-observability.feature:23-24`：provider trace 闸门，关闸零/近零开销。

### 1.3 会话持久化 / 协议面（不同语义族，防误统一）

- 尾部半行截断：`llmanspec/specs/agent-session-store/agent-session-store.feature:111`、`:243`、`:341`（崩溃安全语义，非内容截断）。
- journal 截断 → resync：`llmanspec/specs/protocol-app/protocol-app.feature:265`、`llmanspec/specs/server-core/server-core.feature:114`；JSON 深度 2048 上限：`llmanspec/specs/protocol-app/protocol-app.feature:305`、`llmanspec/specs/server-core/server-core.feature:624`。
- stop_reason 长度截断映射：`llmanspec/specs/infra-provider/infra-provider.feature:105`、`:114`（长度截断 → 上下文溢出路径）。
- bashExecution done 条目 output 截断视图、字节稳定：`llmanspec/specs/agent-session/agent-session.feature:214`；阶段名「终端安全归一化与 UTF-8 安全截断」：`:275`。

### 1.4 轮次/迭代上限（非输出截断）

- `llmanspec/specs/agent-runtime/agent-runtime.feature:189`（should_stop_after_step → AgentEnd reason=StepLimit）、`:417`（LoopMiddlewareLimit/Timeout，32 次硬上限）、`:545` 与 `:557-560`（归因全栈透出）、`:628`/`:637`（插件上限须远小于 32）。
- `llmanspec/specs/runtime-config/runtime-config.feature:168`（回合上限正整数）。
- 拒绝族（不得截断代替）：goal acceptance 4000 字符（`llmanspec/specs/agent-goal/agent-goal.feature:12`、`llmanspec/specs/agent-tools/agent-tools.feature:672`）、todo content ≤80 拒绝写入（`llmanspec/specs/agent-todo/agent-todo.feature:123`）。

### 1.5 端侧呈现面（app-tui-*）

- 脚注呈现：`llmanspec/specs/app-tui-transcript/app-tui-transcript.feature:122-130`（@req:r1345：`[Full output:` 开头脚注行 warning 色；截断须展示脚注不得以 `(truncated)` 字面替换；未截断不得伪造）；`:131-139`（@req:r1346：含 `[Full output:` = 硬截断，Ctrl+O 不得展开全文；ToolExecutionEnd truncated 时以截断后 combined/stdout 替换缓冲）。
- diff 呈现上限：`:140-146`（@req:r1347：可见行数硬上限 + omitted 提示）。
- 历史/窗口：`llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature:260`（单页 200、窗口 800、提示行两态文案钉死）、`llmanspec/specs/app-tui-bridge/app-tui-bridge.feature:169`。
- `…` 类呈现截断：`llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature:15`（footer 放不下截断右侧）、`:298`（阶段名）、`:306`（详情视口不得仅 `…`）；`llmanspec/specs/app-tui-commands/app-tui-commands.feature:82`（resume 预览；同条逐字钉死 busy 通知文案 `Error: ` + `agent busy — finish turn or Esc abort before switching session`）、`:193`。
- 其余：`llmanspec/specs/app-tui-transcript/app-tui-transcript.feature:8`（不得截断历史冒充滚动）、`:387`（args chip header 单行截断）、`:407`（run_code details 呈现）；`llmanspec/specs/app-tui-host/app-tui-host.feature:154`（att14 引用）。

## 2. 两套通知文案的钉死程度（查证问题 2）

**tools 侧 bash 脚注 `[Full output: …]`——三层钉死：**

1. spec 逐字：`infra-bash.feature:24`（@req:r1431）钉死完整模板（含 path/lines shown/limit 与 `(unavailable)` 回退、仅截断时追加）。
2. BDD 步子串：`tests/bdd/steps_shared_thens.rs:132-143` 断言含 `[Full output:` 且含 `lines shown`；`tests/bdd/steps_app_tui_transcript.rs:882` 场景构建器逐字 `[Full output: /tmp/x.log. Truncated: 20 lines shown (50.0KB limit)]`。
3. TUI 识别键：`app-tui-transcript.feature:124`/`:133` 按 `[Full output:` **前缀**识别（着色 + 禁展开行为）。

实现：`plugins/xylitol-official-plugin-tools/src/tools/accumulator.rs:141-159`（display_content 模板）、`plugins/xylitol-official-plugin-tools/src/tools/truncate.rs:2`（DEFAULT_MAX_BYTES=50*1024）、`plugins/xylitol-official-plugin-tools/src/text.rs:1-19`（format_size → `50.0KB` 风格）。

**mcp 侧 `[truncated: …]`——仅语义钉死：**

- spec 只要求「标注保留与丢弃字节数及上限」（`infra-mcp.feature:80`），未钉字串；BDD 步只断 shrink（`tests/bdd/steps_loop_middleware.rs:2053-2062`）；字串 `[truncated: {kept} kept, {dropped} dropped of {total} bytes; limit {limit} bytes]` 仅存在于 `plugins/xylitol-official-plugin-mcp/src/truncate.rs:32-38`，其 `[truncated:` 断言在插件单测 `:109`。
- 其余代码侧通知（非 spec 文本、BDD 子串级）：read 剩余行提示（`plugins/xylitol-official-plugin-tools/src/tools/read.rs:174-176`；步断言 remaining/剩余/offset 三选，`tests/bdd/steps_shared_thens.rs:169-181`）；ls `[N entries shown of M total. Use limit=X for more]`（`tools/ls.rs:125-130`；步断言 `entries shown`，`steps_shared_thens.rs:260-263`）；grep/find limit notices（`tools/grep.rs:201-208`、`tools/find.rs:159-162`，无 BDD 断言）；head 截断通用尾注 `[50.0KB limit reached]`（`tools/truncate.rs:125-136`）；grep 单行截断 `... [truncated]`（`tools/truncate.rs:105-119`，spec 未钉）。

**结论：统一文案是否构成合约变更——分三档：**

- 改 **bash 脚注形状** = 合约变更（动 infra-bash @req:r1431 文本 + app-tui-transcript @req:r1345/r1346 识别键 + agent-tools @req:r5/r1147 场景）。
- 改 **mcp 文案字串** = 不是 spec 合约变更（纯代码 + 插件单测），但新形状必须仍含 kept/dropped bytes + limit 三要素，否则违反 @req:r2934 语义。
- **统一为单一同一字符串**：无 spec 变更做不到——r1431 模板含 path + lines shown，r2934 要求 kept/dropped 字节，两形状不相交。可行统一 = 组合式：bash 保留逐字锚行 + 另行追加通用 bytes 标注行（不触碰 r1431，mcp 采用同一标注行）；或全部收敛到新模板 = 需走 propose 改 r1431/r1345。
- 另注：ls/read 通知文案改动须同步 BDD 步断言（tests，非 specs）；StepLimit 归因文案 `Step limit reached` 被 `tests/bdd/steps_agent_runtime.rs:729` 钉死、spec 侧为语义钉死（`llmanspec/specs/app-tui-bridge/app-tui-bridge.feature:26`），非本次截断面。

## 3. 「不得复用观测 IO 常量」的边界（查证问题 3）

- 观测 IO 常数 = `obs_gate.rs:8` 4096 chars（Truncated 档）与 `:10` 65536 chars（Full 档），经 `ObservationIoTier::max_chars()`（`:42-48`）供给 langfuse/otel 导出面；`truncate_observation_text`（`:12-18`）按 **chars** 计数、gated（tier none → 不附着，`infra-otel.feature:81`/`:116`）。
- 该条款（`infra-mcp.feature:80`）钉的是：工具结果进会话历史的上限必须 = 内置工具的 **50 KiB 字节**口径（实现 `MCP_RESULT_MAX_BYTES = 50*1024`，`plugins/xylitol-official-plugin-mcp/src/truncate.rs:5`；内置 `DEFAULT_MAX_BYTES = 50*1024`，`plugins/xylitol-official-plugin-tools/src/tools/truncate.rs:2`）；**不得**拿 4096/65536（或 `ObservationIoTier::max_chars()`）当工具结果上限，也不得把「gated、面向观测导出、chars 语义」的截断与「always-on、面向会话历史、bytes 语义」的截断混用同一配置/常数。
- `crates/xylitol-plugin-sdk/src/ports/session.rs:320-341` `truncate_utf8` 是字节边界安全通用助手（`normalize_stage_name` / 阶段名，`:343-347`、`crates/xylitol-runtime/src/agent/stage_name.rs:72-73`；spec `agent-session.feature:275`）——属可复用的字节安全原语，不是「观测 IO 常量」。

## 4. 必须保留的差异面清单（查证问题 4）

**合约钉死（T3 统一模块不得改变）：**

1. bash：截断**尾部**（r1431）；逐字脚注模板 + `(unavailable)` 回退 + 仅截断时追加；JSON 契约（display_content/truncated/full_output_path，无全量转储，r1147）；UTF-8 字符边界安全（r8）。
2. read：2000 行 / 50KB 双限 + 剩余行提示语义（r1136）；超 10MB 拒绝而非截断（r4）。
3. mcp：50 KiB bytes 上限、不得复用 obs 常数、kept/dropped bytes + limit 标注、多分片保真、错误与非 MCP 工具豁免、有 servers 才装配且可卸载（r2934）；资源缓存单条 256 KiB 独立面（r3278）。
4. 拒绝族不得改为截断：acceptance 4000、todo 80（见 §1.4）。
5. 观测面 chars 语义 + gated 独立性（§1.2）；执行器 1MB 杀进程面（r1428）。
6. read/grep/find 现行为是**头窗**（`truncate_head`，`tools/truncate.rs:45`），spec 未钉方向但 @req:r42 要求「对齐既有 pi 等价语义」——改方向须谨慎评估；bash 的尾窗是钉死的。

**可统一（代码自由面）：**

- mcp 文案字串；grep/find notice 措辞；read 提示具体措辞（改则同步 BDD 步）；`[50.0KB limit reached]` 通用尾注措辞；grep 单行 `... [truncated]`。
- 默认 limit 值：ls 200（`tools/ls.rs:10`）、grep 100（`tools/grep.rs:15`）、find 1000（`tools/find.rs:13`）——spec 只钉 limit 参数行为与超限指示，未钉默认值。
- grep 单行 500 chars 上限（`tools/truncate.rs:3`）——spec 未钉；codemode 默认 100 KiB（`crates/xylitol-plugin-sdk/src/codemode.rs:114`）——spec 只钉选项存在。
- 两个 50 KiB 常数（tools/mcp 各自 50*1024）可合并为单一常数——r2934 只要求同口径值，不要求常数合一。

**漂移观察（提请 T3/主会话注意，未在本票修复）：**

- `agent-tools.feature:456`（@req:r1140）写「配置 max_lines 100」，实现 `OutputAccumulator` 只有字节滚动尾窗（`tools/accumulator.rs:9-19`、`:74-89`），无 max_lines 配置——spec 与代码不一致，未确认何者为期望（查证方法：全 crate grep `max_lines` 仅 read/grep/find 路径使用）。
