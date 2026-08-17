# c2210 live spec 产品级审计表

> 尺子（OpenSpec 同构）：**实现能改、对外行为不变 → 不进 spec。**
> 判定：`keep` 可观察且语句干净；`rewrite-product` 留语义去路径/类型/行数；`move-to-AGENTS` 代码组织/文档职责；`delete` 过期/防复活/空套话/元闸。
> 大组织方向可 keep：分层依赖、端口 seam、crate 边界、组合根职责、跨面同源。代码事实核对于 2026-08 中旬本 worktree（`c2210-spec-audit` @ 5908a898）。

## 批次 1

### valid_scope（5 个批次 1 cap 的路径钉，统一判定）

| capability | req_id | 判定 | 现行摘句（≤40字） | 改写草稿或迁入何处 | 歧义？ |
|---|---|---|---|---|---|
| layer-architecture | valid_scope | move-to-AGENTS | valid_scope[3]: src/,tests/,src/AGENTS.md | 放宽为层范围（agent/infra/app/protocol 层 + tests）；src/AGENTS.md 是迁入目的地而非 spec 哨兵范围 | — |
| app-tui-host | valid_scope | move-to-AGENTS | valid_scope[2]: src/app/tui/,tests/tui_e2e/ | 放宽为「产品 TUI 面 + TUI e2e 测试」 | — |
| app-tui-chrome | valid_scope | move-to-AGENTS | valid_scope[1]: src/app/tui/ | 放宽为「产品 TUI 壳层」 | — |
| app-tui-design-playground | valid_scope | move-to-AGENTS | valid_scope[7]: …playground/,sync_tokens.py,index.html,fixtures/,DESIGN.md | 整 cap 若保留则放宽为「design playground 与视觉 SSOT」；7 个文件路径钉由 lint（adp6）与 design AGENTS 承继 | — |
| package-tui-testing | valid_scope | move-to-AGENTS | valid_scope[2]: packages/xylitol-tui/,tests/ | 放宽为「xylitol-tui 包 + workspace tests」 | — |

### layer-architecture

| capability | req_id | 判定 | 现行摘句（≤40字） | 改写草稿或迁入何处 | 歧义？（计数/时态/身份是否未钉） |
|---|---|---|---|---|---|
| layer-architecture | la1 | rewrite-product | System MUST 以 src/agent/ 承载编排与投影…单一 src/protocol/…禁止 src/domain/ | 「System MUST 以 agent 层承载编排与投影；跨边界契约集中于单一 protocol 层（wire 线协议 + ports 端口；wire ↛ ports；进签名的共享类型位于 protocol 根或领域聚子树）；MUST NOT 提供独立 domain / runtime_protocol / vocab-types 第三顶栏」——删全部 `src/` 路径与 `ReAct/project_for_llm` 类型名 | 无 |
| layer-architecture | la2 | rewrite-product | infra/ 模块 MUST NOT import crate::agent…app/core seam（composition/Driver/bootstrap/dispatch） | 保留依赖方向图（infra ↛ agent、agent ↛ infra、protocol ↛ agent/infra、面经 seam）；删 `crate::agent`/`composition.rs` 等路径与「mod 级导出」表述，改写为「经公共入口导入」 | 无 |
| layer-architecture | la4 | rewrite-product | 表面与跨面缝分离：缝位于 app/core/；print MUST 位于 app/cli/ | 「app 表面（用户入口）与跨面缝（组合根、driver）MUST 分离；print MUST 为 CLI 子模式而非顶层 app 模块」——删 `app/core/`、`app/cli/` 路径 | 无 |
| layer-architecture | la6 | rewrite-product | System MUST 在 src/app/server/ 提供 app::server 模块 | 「System MUST 提供 server 应用面，托管 agent + infra 运行时并经 REST（/api/v1）与 WebSocket 暴露 protocol；server MUST 强制单实例锁」——server 面存在性与单实例锁是可观察契约，keep；删路径 | 无 |
| layer-architecture | la7 | rewrite-product | 全部运行时 MUST 位于 infra/；agent/ 仅以 Arc<dyn Port> 注入 | 「运行时能力（provider、tool、session storage、exec-env 等）MUST 位于 infra 层；agent 层 MUST 仅经 protocol 端口注入持有，MUST NOT 直接调用 infra 构造器」——删 `Arc<dyn Port>` 类型细节 | 无 |
| layer-architecture | la8 | keep | Agent 构造器 MUST NOT 要求 Session…session_id 为 app 层绑定 | 组合根职责（session 绑定在 app 层），属大组织方向例外。已干净 | 身份：`Agent` vs `AgentRuntime` 未钉（主语模糊），建议定「能力聚合体构造路径」 |
| layer-architecture | la9 | move-to-AGENTS | 分层不变量 MUST 经 src/AGENTS.md、write-surface 流程与缝行为测保障 | 保障手段 = 仓库自身约定：迁 `src/AGENTS.md`（「分层保障：AGENTS + write-surface + 缝行为测；禁止源码 grep 元测试强制闸」）。依赖方向半句已由 la2 承载，此处可删 | 无 |
| layer-architecture | la13 | keep | agent/ MUST NOT 托管纯展示或用户指引逻辑 | 层职责边界（展示归 app），大组织方向。已干净（`agent/` 可写作「agent 层」） | 无 |
| layer-architecture | la14 | delete | MUST NOT 保留独立 src/domain/ 顶栏、pub use shim…调用点 MUST 一次性迁到 | 防复活 + 迁移清单：`src/domain/` 已不存在（verified），迁移早已完成。Xy 前缀归属半句已由 ar06/la1 覆盖 | — |
| layer-architecture | la15 | rewrite-product | MUST 提供 app::core::composition 模块，从 BuildAgentOptions 集中构造 | 「System MUST 提供单一组合根装配入口，集中构造完整接线的 Agent，使 CLI、Server 与 TUI 不重复接线代码」——组合根职责保留，删模块路径 | 无 |
| layer-architecture | la17 | rewrite-product | protocol/ports/ MUST 托管 Xy 前缀端口契约；签名共享类型来自 protocol 根 | 「Xy 前缀端口契约 MUST 位于 protocol 层端口区；其签名所用共享类型 MUST 来自 protocol 层根共享区」——端口 seam 保留，删路径 | 无 |
| layer-architecture | la19 | rewrite-product | app/ MUST 将跨面缝分组于 src/app/core/…MUST NOT 直接托管 build_agent 接线或 Driver trait | 「app 层 MUST 将跨面缝（组合根与运行时 driver 边界）与自包含应用表面分离；应用表面 MUST NOT 直接托管 build_agent 接线或 Driver trait」——删路径与类型名（或改「Driver 或等价」） | 无 |
| layer-architecture | la20 | rewrite-product | app::core MUST 托管 bootstrap（config load、ModelRegistry build…）与 dispatch | 「System MUST 提供共享 bootstrap 入口集中完整装配路径，与共享 dispatch 入口集中 Command 执行语义；print、server、tui MUST 调用 bootstrap，tui MUST 调用 dispatch；任何表面 MAY NOT 内联自有装配或分发」——组合根职责保留；括号内装配清单 = 迁移清单，迁 `src/AGENTS.md` | 无 |
| layer-architecture | la21 | move-to-AGENTS | agent/ 内纯转发壳 MUST 删除；MUST NOT 保留 SessionIO 类零逻辑包装 | 死码/体量策略：迁 `src/AGENTS.md` 体量段；`SessionIO` 防复活半句删 | 无 |
| layer-architecture | la-embed1 | keep | MUST 向库用户提供文档化嵌入入口…仅依赖精选 Xy 契约 | 下游（库用户）可观察；「bootstrap/Driver 或等价」已有逃生阀 | 无 |
| layer-architecture | la-server-driver | keep | Server 与 Print MUST 共享同一应用缝（Driver/bootstrap） | 跨面同源，语句已干净 | 无 |
| layer-architecture | la-mcp-seam | keep | MCP 描述经嵌入缝传递；MUST NOT reach-in infra::mcp | 端口 seam（大组织方向）。`composition/McpSession` 类型名建议改「或等价」 | 无 |
| layer-architecture | la-dispatch-consume | move-to-AGENTS | dispatch MUST 至少被一个生产应用面消费；禁止仅单测覆盖 | 死码闸（逻辑死），非用户可观察：迁 `src/AGENTS.md`（或并入 audit-dead-code skill 分诊规则） | 无 |
| layer-architecture | la25 | keep | 厂商差异收敛在 xylitol-ai-bridge adapter 族…开闭扩展 | crate 边界 + 开闭规则，大组织方向。已干净 | 无 |
| layer-architecture | la26 | keep | 每个可选能力定义带域前缀 feature flag；内置能力无 flag | 下游可观察（cargo feature 是库契约）。`infra-acp` 具体命名可随组织演化 | 无 |
| layer-architecture | la27 | keep | Cargo.toml default MUST 为 cli + tui + otel | 下游可观察构建契约，已干净 | 无 |
| layer-architecture | ar01 | move-to-AGENTS | src/ 中所有源文件 MUST NOT 在 doc comments 中引用 pi coding agent | 文档卫生：迁 `src/AGENTS.md`。注：当前代码仍有 85 处 doc comment 含 pi 引用（如 `hooks.rs` "pi-aligned"）——迁出 spec 不等于豁免，建议另票清理 | 无 |
| layer-architecture | ar06 | keep | 精选进 crate 公开 API 的可替换端口…MUST 使用 Xy 前缀 | 库公共 API 命名契约，下游可观察。已干净 | 无 |
| layer-architecture | ar07 | keep | Usage、StopReason 与 Compaction 配置 MUST 各有一个 canonical 类型 | 数据契约（单类型），下游可观察。已干净 | 无 |
| layer-architecture | ar09 | keep | MUST 在 src/lib.rs（或等价公开入口）维护精选 pub use | 库公共 API 契约；「或等价公开入口」已有逃生阀 | 无 |
| layer-architecture | r12 | rewrite-product | JSON Schema DTO MUST 位于 infra/config 或 infra/settings… | 「需要 JSON Schema 的配置/settings DTO MUST 收敛于配置边界；会话词汇 MUST NOT 仅为 schema 而 derive JsonSchema」——后半句（会话词汇不因 schema 派生）是真实契约，保留；删模块路径 | 无 |

### app-tui-host

> 补充：`app-tui-host.feature:69` 步骤「假如 打开 src/app/tui/AGENTS.md」（@req:ath10 场景）随 ath10 迁出，场景删除或改为行为断言。

| capability | req_id | 判定 | 现行摘句（≤40字） | 改写草稿或迁入何处 | 歧义？（计数/时态/身份是否未钉） |
|---|---|---|---|---|---|
| app-tui-host | ath1 | rewrite-product | 使用 dispatch_event、request_render、try_render、idle_tick；MUST NOT 调用 TUI::start() | 「产品 TUI MUST 以 host 驱动引擎：事件分发、按需渲染、idle 心跳，MUST NOT 进入引擎的阻塞主循环」——保留同步驱动语义，删 API 名（host 与包的绑定 API 属实现） | 无 |
| app-tui-host | ath2 | keep | 正常退出、错误退出与 panic 路径恢复终端；安装 panic hook | 用户可观察。已干净 | 无 |
| app-tui-host | ath3 | rewrite-product | debug 默认启用即时文件日志…src/app/tui/AGENTS.md MUST 写明日志路径 | 「debug 构建下产品 TUI MUST 默认启用即时文件日志（可 tail -f 观察）；release MUST 默认关闭，经 XYLITOL_DEBUG 或 RUST_LOG 显式打开」——保留可观察行为；「AGENTS.md 写明路径」半句迁面 `src/app/tui/AGENTS.md` | 无 |
| app-tui-host | ath4 | keep | resize 触发重绘；宽<40 或高<6 显示友好提示而非 panic | 用户可观察；阈值是产品行为。已干净 | 无 |
| app-tui-host | ath5 | rewrite-product | 事件推进与终端 I/O 解耦，使 min-size 提示…可在无真 TTY harness 验证 | 「产品 TUI host MUST 将事件推进与终端 I/O 解耦（可注入事件与终端），使关键路径可在无真 TTY harness 验证」——可测性 seam 保留，删 HostEvent/TestTerminal 类型名 | 无 |
| app-tui-host | ath6 | keep | 经单一 drain_pending（或等价共享入口）消费…harness 泵 MUST 复用同一入口 | 生产/验收同源（跨面同源例外），「或等价」有逃生阀 | 「与生产循环同序同分支」——同序的验证口径未钉（建议附测试断言语义） |
| app-tui-host | ath7 | rewrite-product | run_host_loop 与 harness 调用同一共享 bang/主环事件臂 | 保留行为（bang 中 Esc 可达 abort；chunk 仅标记 dirty；bang 不饿死 agent）；删 `run_host_loop`/`Driver::abort`/`note_bash_cancelled` 名，改「Driver 或等价」 | 无 |
| app-tui-host | ath8 | keep | Esc 同步步进内臂装 suppress_xy_until_stream_end（或等价门闩） | 用户可观察（abort 后 delta 不复活 busy）；「或等价」有逃生阀 | 无 |
| app-tui-host | ath10 | move-to-AGENTS | src/app/tui/AGENTS.md MUST 提供 kimi 式本地布局地图 | 整条是「AGENTS.md 应写什么」的自指要求，属面 AGENTS 内容本身（host/mod、effects/bridge 等切分在 ath12 同样迁）。另有同主题 feature 场景（`app-tui-host.feature` 「打开 src/app/tui/AGENTS.md」）须同步改 | 无 |
| app-tui-host | ath11 | rewrite-product | Busy Esc MUST 在 HostSession::step 内同步臂装 Xy 抑制 | 「Busy Esc（非 overlay）MUST 在同一同步步进内臂装 Xy 抑制，MUST NOT 仅依赖下一轮循环顶部才开始丢弃；bang Esc 路径 MUST 继续走取消语义，不得与 agent abort 文案混用」——删 `HostSession::step` 类型名 | 无 |
| app-tui-host | ath12 | move-to-AGENTS | host/mod.rs、layout/root/mod.rs…cognitive ≤35 且 cyclomatic ≤30…800/1200 行 | 文件切分 + 复杂度闸 + 行数软顶 = 代码组织：迁 `src/AGENTS.md`（体量策略已有落点）；seam 半句（组件 MUST NOT 直接调用 Driver）由 ath14/15/16/21/23 覆盖。与 test-qa-gate qg06、`src/app/tui/tests.rs` ath12_entry_files_under_hard_smell_loc、`scripts/check_complexity.py` 四处双写，迁出后统一指向 AGENTS | 无 |
| app-tui-host | ath13 | keep | get_messages 失败 MUST 展示 system/error note，不得静默空 transcript | 用户可观察；「或等价」有逃生阀 | 无 |
| app-tui-host | ath14 | keep | 会话列表经 Driver seam；行支撑 scope/cwd/预览/parent/mtime | 用户可观察 + seam（大组织方向） | 无 |
| app-tui-host | ath15 | keep | new_session/get/set_session_name 经 seam；CR/LF 规范为空格并 trim | 用户可观察 + seam | 无 |
| app-tui-host | ath16 | keep | Resume 面板 rename/delete 经 seam；删除确认；scope=Current 语义 | 用户可观察 + seam | 无 |
| app-tui-host | ath20 | keep | /reload 经缝调用 skills/MCP/keybindings/theme 重载；失败继续其余步骤 | 用户可观察（部分成功语义） | 无 |
| app-tui-host | ath21 | keep | /history-copy-last 经 Driver 缝；取已提交 Assistant 首条非空正文 | 用户可观察 | 无 |
| app-tui-host | ath22 | keep | 切换 model/thinking 成功不写滚动提示；pending 走 atc19 | 用户可观察 | 无 |
| app-tui-host | ath23 | keep | loaded-resources 经 Driver 只读缝；MCP 未结算不拒键入 | 用户可观察 | 无 |
| app-tui-host | ath24 | keep | Ready 态 Tick 仅 dirty 才重绘；上区行缓存失效条件 | 可测行为（重绘计数上界）；「status Loader 推进」边界已列举 | Loader 帧动画推进是否算「纯 status 变化」未钉（建议补：动画帧推进不失效上区） |
| app-tui-host | ath25 | keep | 已提交 UiEntry 按条目指纹行缓存；可测重绘/miss 上界 | 可测行为 | 无 |
| app-tui-host | ath26 | keep | streaming Markdown 前缀复用；可测全量解析计数上界 | 可测行为 | 无 |
| app-tui-host | ath27 | keep | MCP 发现接 /mcp 列表；短 cue 文案固定且右对齐 | 用户可观察（文案固定） | 无 |
| app-tui-host | ath28 | keep | reload 进行中态独立；Enter 拒绝 body 固定；取消收口快照一致 | 用户可观察 | 无 |
| app-tui-host | avs1 | keep | ScriptedDriver 合成验收覆盖提交→流式→abort→/exit（H1–H9） | 验证契约（test-* 域可 keep） | H1–H9 是测试名引用，harness 重命名会漂移——建议改「全链场景清单」叙述 |
| app-tui-host | avs2 | rewrite-product | tests/tui_e2e MUST 包含产品 PTY 冒烟（#[ignore]，经 just test-tui-e2e-pty） | 「仓库 MUST 含产品二进制的 PTY 冒烟（#[ignore]，经 just test-tui-e2e-pty 可跑）：Fake 模型 + --trust 下提交消息屏上出现 Hello from fake provider，/exit 后进程退出；默认 just qa 不强制」——删 `tests/tui_e2e` 路径钉 | 无 |
| app-tui-host | ath29 | keep | Mouse 映射 opt-in；无态变 Moved 不重绘；teardown 关 capture | 用户可观察 | 无 |
| app-tui-host | ath30 | keep | 交互模式默认 ApplicationOwned；会话内不热切 | 用户可观察 | 无 |
| app-tui-host | ath31 | keep | 松手复制成功提示 TTL 1.5–3s；不写 transcript | 用户可观察 | 无 |
| app-tui-host | ath32 | keep | 主输入 Enter 滚底并恢复尾随；空输入不创建 submit | 用户可观察 | 无 |
| app-tui-host | ath33 | keep | 折叠三角命中吞按并单块 toggle；拖选中不 toggle | 用户可观察 | 无 |

### app-tui-chrome

| capability | req_id | 判定 | 现行摘句（≤40字） | 改写草稿或迁入何处 | 歧义？（计数/时态/身份是否未钉） |
|---|---|---|---|---|---|
| app-tui-chrome | atc1 | keep | 空闲 status 0 行；忙碌至多独立 1 行；不并入 footer | 用户可观察，已干净 | 无 |
| app-tui-chrome | atc2 | rewrite-product | footer 1 行 dim；字段序 cwd·model·thinking；used … tokens | 几乎全干净（可观察文案与字段序）。仅「level 文案为 ThinkingLevel::as_str」钉类型名 → 改「level 文案为该档位的显示名」；`TokenProvenance` 枚举名可留（数据契约）或改「来源类别」 | 无 |
| app-tui-chrome | atc3 | keep | Palette::dark()（或等价闭包）注入；glyph 档显式配置切换 | 「或等价」有逃生阀；运行时字体探测禁令是行为 | 无 |
| app-tui-chrome | atc4 | rewrite-product | 视觉与 UX MUST 以 src/app/tui/DESIGN.md 为索引、design/*.md 为组件级 SSOT | 「产品 TUI 视觉与 UX MUST 有单一书面 SSOT（总索引 + 组件级子文档）；实现与 demo 引用规则 MUST 指向对应文档，MUST NOT 仅依赖口头约定」——删路径；文档落点（DESIGN.md/design/）写入面 AGENTS | 无 |
| app-tui-chrome | atc5 | keep | agent_demo 默认 Dark；自动探测经纯函数切换且非产品默认 | demo 面行为 + 跨面同源禁令。可留（或迁 package-tui-agent-demo） | 无 |
| app-tui-chrome | atc6 | keep | 空闲空草稿操作区紧凑；多行草稿按规则长高 | 用户可观察 | 无 |
| app-tui-chrome | atc7 | keep | busy status 一行：spinner+短词贴左，不 space-between | 用户可观察 | 无 |
| app-tui-chrome | atc8 | keep | 用户消息行默认不淡底；保留语义前缀 | 用户可观察 | 无 |
| app-tui-chrome | atc9 | keep | Compacting/Retry 态 status 仍一行；细节走 ScrollNotice | 用户可观察 | 无 |
| app-tui-chrome | atc10 | keep | abort 后 status 回 0 行；取消说明走滚动提示 | 用户可观察 | 无 |
| app-tui-chrome | atc11 | keep | models 槽切换后 footer 反映 active；pending 仅下轮预告 | 用户可观察 | 无 |
| app-tui-chrome | atc12 | keep | busy Loader 前导空行+紧贴 editor；idle 保留一行呼吸间距 | 用户可观察 | 无 |
| app-tui-chrome | atc13 | keep | TokenProvenance 文案：Api 无波浪号，Heuristic 加 ~，Unknown 加 ? | 用户可观察文案（数据契约） | 无 |
| app-tui-chrome | atc14 | keep | token 字段刷新触发点；同一轮不二次 estimate | 可测行为（含节流） | 无 |
| app-tui-chrome | atc15 | keep | 主题热应用；未知名保留旧主题并诊断；不重载 transcript | 用户可观察 | 无 |
| app-tui-chrome | atc16 | keep | /theme 无参独立 editor 槽；Esc 关闭不改主题 | 用户可观察 | 无 |
| app-tui-chrome | atc17 | keep | thinking 档映射边框色；bash 强调色覆盖与恢复 | 用户可观察 | 无 |
| app-tui-chrome | atc18 | keep | loaded-resources 槽内容与 MCP connecting 进度；无木糖醇标签 | 用户可观察 | 无 |
| app-tui-chrome | atc19 | keep | 下轮预告右对齐；model 优先文案；仅 bang busy 不显示 | 用户可观察（文案已钉） | 「NextTurn 消费」的定义未钉（谁消费、何时清）——建议补一句 |
| app-tui-chrome | atc20 | keep | 成功切换不写滚动确认；失败/诊断 MAY 写 | 用户可观察 | 无 |
| app-tui-chrome | atc21 | keep | 派生占用比 1 位小数；Heuristic ~p%；仅展示非闸 | 用户可观察文案 | 无 |
| app-tui-chrome | atc22 | keep | 壳层通告：warning 前景、`Error: ` 前缀、单槽 TTL 3–5s | 用户可观察 | 无 |
| app-tui-chrome | atc23 | keep | Chrome Footprint 预留行预算；短终端仍见 status lead | 用户可观察（预算公式是行为） | 无 |
| app-tui-chrome | atc24 | rewrite-product | C 与 window 共用同一算法（产品 format_compact_tokens / 对齐 pi） | 阈值表（<1000 全量、千位 1 位小数 k、万位整 k、M）是可观察文案契约，保留；删函数名 `format_compact_tokens` →「同一紧凑算法（实现单点，勿另立第二套阈值）」 | 「四舍五入整 k」的舍入方式已钉（round）；`?` 不套紧凑已钉 |
| app-tui-chrome | atc25 | keep | reload 态 status Loader 形；toast body 固定三文案 | 用户可观察 | 无 |
| app-tui-chrome | atc26 | rewrite-product | 壳层文案仅来自 DESIGN.md 与 design/*.md 已声明槽 | 「壳层可观察文案与徽章 MUST 仅来自文档化视觉 SSOT 已声明槽与字段（见 atc4），MUST NOT 另加未声明冗余展示」——反冗余语义保留，删路径 | 无 |

### app-tui-design-playground

| capability | req_id | 判定 | 现行摘句（≤40字） | 改写草稿或迁入何处 | 歧义？（计数/时态/身份是否未钉） |
|---|---|---|---|---|---|
| app-tui-design-playground | adp1 | rewrite-product | tokens.css/js MUST 由 sync_tokens.py 从 DESIGN.md frontmatter 生成 | 「playground token 生成物 MUST 由单一生成脚本从视觉 SSOT 生成；手改生成物 MUST NOT 作为长期真值」——生成物 SSOT 不变量保留；脚本名/落点进 design AGENTS（lint 已把关） | 无 |
| app-tui-design-playground | adp2 | move-to-AGENTS | playground Markdown 槽示意 MUST 对齐 design/markdown.md | 静态示意细节 = design 文档职责（`check_tui_design_playground.py` 已入 qa 把关） | 无 |
| app-tui-design-playground | adp3 | move-to-AGENTS | design/AGENTS.md MUST 写明 Agent 默认忽略 playground/ | 自指要求：这条 statement 本身就是 design/AGENTS.md 的既有内容（verified：已有「默认忽略 playground/」行） | 无 |
| app-tui-design-playground | adp4 | move-to-AGENTS | playground 会话树槽静图 MUST 用 DESIGN token 色区分 kind 前缀 | 静图内容一致性 → design 文档/lint 职责 | 无 |
| app-tui-design-playground | adp5 | move-to-AGENTS | 注脚与键位示意 MUST 写明活树与 travel_session_tree | 同上（lint 已断言） | 无 |
| app-tui-design-playground | adp6 | keep | scripts/check_tui_design_playground.py（或等价）经 just qa 入闸非零退出 | qa 验证契约（test-* 域可 keep）；「或等价」有逃生阀 | 无 |
| app-tui-design-playground | adp7 | move-to-AGENTS | src/app/tui/design/fixtures/ MUST 存在 session-tree.filter… | fixture 清单属 lint/design 维护（adp6 已断言存在性）；删路径钉 | 无 |
| app-tui-design-playground | adp8 | move-to-AGENTS | design/AGENTS.md 与 playground README MUST 写明改静图须跑 lint | 文档指针要求 → design AGENTS/README 自身职责 | 无 |
| app-tui-design-playground | adp9 | move-to-AGENTS | playground MUST 提供静态槽 panel-models、panel-tree-power… | 静图槽形状 → design 文档/lint 职责 | 无 |
| app-tui-design-playground | adp10 | delete | 下一波（/models、树 filter/fold/fork、真 EDITOR、footer context%、abort UI）视觉以 DESIGN.md Next wave 表为 SSOT | 过期 Next-wave 名单：清单内能力均已落地为 app-tui-* live spec（/models→atm1、fold→att20/21、EDITOR→ati17、footer%→atc21、abort UI→ati15 族）。视觉 SSOT 指针由 atc4 改写稿承载 | — |

### package-tui-testing

| capability | req_id | 判定 | 现行摘句（≤40字） | 改写草稿或迁入何处 | 歧义？（计数/时态/身份是否未钉） |
|---|---|---|---|---|---|
| package-tui-testing | tt01 | keep | 五层测试架构：按键序列/insta/时序/proptest/PTY+tmux | 测试基础设施 capability 的交付物本体，可 keep | 无 |
| package-tui-testing | tt02 | rewrite-product | MutableComponent 泛化到 tests/support/；不重建第二实例（virtual_terminal_test.rs:178-184） | 行号引用已过期（verified：现 178-184 是差分渲染测试；变通已消失，support/mod.rs:869 的 TuiTestHarness 即「同实例跨帧」）。改写：「通用按键序列驱动（字节串分发 + assert_text/assert_cursor/assert_screen_contains）；辅助 MUST 原地变更同一实例，MUST NOT 重建第二实例观察后续帧」——删 tests/support 路径与行号句 | 无 |
| package-tui-testing | tt03 | keep | insta assert_snapshot! 整屏黄金 + assert_eq pinpoint 共存 | 交付物契约，已干净 | 无 |
| package-tui-testing | tt04 | keep | 可注入 Clock；禁止 thread::sleep；paste-burst 边界显式覆盖 | 交付物契约，已干净（8ms/120ms 边界两侧已钉） | 无 |
| package-tui-testing | tt05 | keep | portable-pty 为主驱动；位于 tests/tui_e2e/（非包内） | 交付物契约；落点说明是测试组装性质（可留，或把「因组装二进制而非 crate」理由挪包 AGENTS） | 无 |
| package-tui-testing | tt06 | keep | tmux 冒烟 3-5 例 #[ignore]；经 test-tui-e2e 目标跑 | 交付物契约，已干净 | 无 |
| package-tui-testing | tt07 | keep | 单一主 example 表面；覆盖 transcript/tool/侧栏/输入区 | 包公共交付物契约，已干净 | 无 |

## 批次 2（按 capability 汇总，仅展开明显违规 req）

| capability | req_id | 判定 | 现行摘句（≤40字） | 改写草稿或迁入何处 | 歧义？ |
|---|---|---|---|---|---|
| agent-hooks | （cap 汇总） | keep | valid_scope 钉 src/infra/hooks/ 与 src/agent/runtime/hooks.rs | 18 条 req 全为 hook 链可观察语义（可拒绝、after 链、只影响下一轮），已干净；valid_scope 路径钉放宽为「agent 层 + infra 层」（下同：65 个 cap 的 valid_scope 逐 cap 放宽，不逐条展开） | 无 |
| agent-prompt | （cap 汇总） | keep | pt1/pt5/pt9/pt10 系统提示组装语义 | 语句干净（沙箱渲染、白名单、替换语义均行为级）；pt5 的 minijinja 属技术选型但由「沙箱白名单」行为承载，可留 | 无 |
| agent-runtime | ar13 | delete | src/ MUST NOT 使用 r#loop 标识符；循环模块名为 runtime::react | 防复活命名禁令（verified：代码已无 r#loop）；迁 src/AGENTS.md 或删 | — |
| agent-runtime | ar-pilot | delete | agent-runtime spec MUST 在 …/agent-runtime.feature 携带 react-terminates 与 stream-is-xyevent | 试点期自指 req（feature 已存在且绿）；通用规则在 test-bdd tb3 | — |
| agent-runtime | ar3/ar12/ar21/ar24/ar27/ar32 | rewrite-product | …该行为 MUST 有可执行 BDD 场景（live .feature, @req） | 各 req 的行为半句全部可留；删句尾「MUST 有可执行 BDD 场景」元句（BDD 组织由 llmanspec/AGENTS + test-bdd 管，不逐 req 重复） | 无 |
| agent-session | a10 | delete | agent-session.feature 下 BDD 测试 MUST 全部通过 | 元闸 req（CI 已保证）；同型：agent-session-store s8/s17、domain-compaction c6、agent-tools t13/t16 一并删 | — |
| agent-session | （其余） | keep | 工具事件时序、模型切换钳制等 | 行为级干净 | 无 |
| agent-session-store | s20 | keep | SESSION_VERSION(5) 不符拒绝；warn≤3 后省略收敛 | 数据契约（会话文件格式版本与容错）已钉计数 | 无 |
| agent-todo | （cap 汇总） | keep | Todo 快照与 checklist 折叠 | 行为级干净 | 无 |
| agent-tools | （cap 汇总） | keep | 文件类工具与批执行语义 | 除 t13/t16 元闸外干净 | 无 |
| app-tui | tui5 | move-to-AGENTS | src/app/tui/ 下每个新增源文件 MUST 同一变更内由真实入口驱动；禁 #[allow(dead_code)] 骨架 | 死码/交付卫生：迁 src/AGENTS.md（audit-dead-code skill 分诊规则） | 无 |
| app-tui | tui4 | rewrite-product | TUI MUST 仅从 crate::agent（mod 级）、crate::app::core、crate::protocol 导入 | 「产品 TUI MUST 经协议契约、应用缝与 agent 公共入口导入，MUST NOT reach agent 子模块内部或 infra；斜杠语义 MUST 复用 protocol::Command」——seam 保留，删具体 crate 路径 | 无 |
| app-tui | tui2/tui3/tui-index | keep | InProcessDriver 默认、三面共存、capability 分工索引 | 组合根/跨面同源 + capability 索引（spec 组织必需品） | 无 |
| app-tui-ask | （cap 汇总） | keep | ask 工具槽、skip/answered 语义 | 行为级干净 | 无 |
| app-tui-bridge | （cap 汇总） | keep | UiModel 投影 14 条 | 行为级干净 | 无 |
| app-tui-commands | （cap 汇总） | keep | 斜杠与 dispatch 映射 | 行为级干净（atm2 的 dispatch 共享是跨面同源） | 无 |
| app-tui-input | （cap 汇总） | keep | 输入/槽/补全/外部编辑器 | 行为级干净（ati17 等已用「或等价」） | 无 |
| app-tui-session-tree | ast9 | rewrite-product | MUST NOT 在 src/app/tui 重写 folded_nodes 算法 | 「MUST NOT 在应用面重写折叠算法（复用包 TreeSelector 语义）」——删路径 | 无 |
| app-tui-session-tree | （其余） | keep | 树/旅行/标签行为 | 行为级干净 | 无 |
| app-tui-transcript | （cap 汇总） | keep | 33 条投影/折叠/簇头 | 行为级干净（att5/att11 有「或等价包 API」逃生阀；att24 计数已钉：Ran N=次数、Used N=调用次数、文件 N=去重 path） | 无 |
| app-tui-trust | （cap 汇总） | keep | ChoicePrompt 信任选择 | 行为级干净 | 无 |
| cli-entry | ce1 | rewrite-product | CLI 模块 MUST 位于 src/app/cli/；app 面 MUST NOT reach agent 内部 | 「CLI MUST 为独立应用表面，print 为其子模式；app 面 MUST NOT reach agent 内部（经缝）」——删路径钉，seam 半句保留 | 无 |
| cli-entry | ce11 | delete | stdio RPC（rpc.rs / --rpc / feature）MUST 保持移除 | 防复活（对象已移除）；已删除引用条款随删除清理 | — |
| cli-entry | （其余） | keep | ce2/ce17 fail-closed、ce20 resume-hint 等 | 行为级干净（ce15 的 tokenizer 委托 bridge 是 crate 边界，可留） | 无 |
| cli-print | （cap 汇总） | keep | print 输出语义 5 条 | 行为级干净；valid_scope 钉 src/app/cli/print.rs 放宽 | 无 |
| domain-compaction | （cap 汇总） | keep | 压缩策略 27 条 | 行为级干净（除 c6 元闸删） | 无 |
| domain-security | r71 | delete | src/agent/trust/（store.rs、resolve.rs）重复实现 MUST 移除 | 防复活 + 迁移清单（verified：src/agent/trust 不存在）。第一半句「infra 单一 trust 真源」是 seam，保留并入 r61 族或改写 | — |
| domain-security | （其余） | keep | 网络域强制等 | 行为级干净 | 无 |
| infra-bash | （cap 汇总） | keep | bash 执行器语义（truncate/取消/上下文过滤） | 行为级干净 | 无 |
| infra-clipboard | （cap 汇总） | keep | 剪贴板平台顺序与 OSC52 限制 | 行为级干净 | 无 |
| infra-diagnostics | （cap 汇总） | keep | timing 收集由 env 门控 | 行为级干净 | 无 |
| infra-git | （cap 汇总） | keep | repo/branch/URL 检测 | 行为级干净 | 无 |
| infra-image | （cap 汇总） | keep | 缩放/EXIF/格式转换 | 行为级干净 | 无 |
| infra-mcp | （cap 汇总） | keep | MCP 连接/定稿门闸（mcp8 计数已钉） | 行为级干净；valid_scope 钉 4 个文件路径放宽为「infra 层 + app 缝」 | 无 |
| infra-network | nc0 | delete | System SHALL 支持可配置的 HTTP 代理与 idle 超时设置 | 空 placeholder（SHALL 无主体；nc1–nc5 已覆盖全部内容） | — |
| infra-network | （其余） | keep | 代理/超时配置 | 行为级干净 | 无 |
| infra-observability | dl1 | rewrite-product | 组合根（app::cli::run，CliArgs::parse 之后）恰好一次装配 | 「观测后端 MUST 恰好一次装配；级别 log 与 provider trace 只写日志目录文件（0o600），MUST NEVER 写 stdout/stderr；debug 默认开、release 经 XYLITOL_DEBUG/XYLITOL_PROVIDER_TRACE 显式开」——删组合根路径/类型名 | 无 |
| infra-observability | ipt3 | move-to-AGENTS | Cargo 与 src MUST NOT 依赖 tracing/tracing-subscriber | 技术栈禁令 → src/AGENTS.md（观测栈单一化：fastrace + log 门面）；ipt1/ipt2/ipt4 行为可留 | 无 |
| infra-otel | （cap 汇总） | keep | OTLP opt-in 出口 22 条 | 行为级干净；valid_scope 5 路径放宽 | 无 |
| infra-process | （cap 汇总） | keep | bash 发现/进程组/等待 | 行为级干净 | 无 |
| infra-provider | （cap 汇总） | keep | provider 适配与路由 | 行为级干净 | 无 |
| package-ai-bridge | pab19 | keep | WirePolicy 默认值 SSOT（defaults.rs 或等价纯常量模块） | 数据契约 + 「或等价」逃生阀；scenario 再点名 defaults.rs 可改为「默认板所在常量模块」 | 无 |
| package-ai-bridge | （其余） | keep | DTO/适配器行为 | 行为级干净 | 无 |
| package-ai-bridge-accounting | （cap 汇总） | keep | 用量/计费语义 | 行为级干净 | 无 |
| package-tui-agent-demo | （cap 汇总） | keep | demo 面行为（pad6 的 Shift+Tab cycle） | 行为级干净；valid_scope 两文件路径放宽 | 无 |
| package-tui-autocomplete / editor-autocomplete | （cap 汇总） | keep | 补全行为 | 行为级干净 | 无 |
| package-tui-choice-prompt | （cap 汇总） | keep | 选择槽行为 | 行为级干净 | 无 |
| package-tui-diff | （cap 汇总） | keep | diff 渲染 | 行为级干净 | 无 |
| package-tui-editor | ed03 | rewrite-product | MUST 用 build_visual_line_map 实现 pageScroll | 「pageScroll 按宽度相关视觉行布局实现，page size = max(5, terminal_rows*30/100)，光标按 page 移动视觉行」——删内部函数名（改「或等价」） | 无 |
| package-tui-editor | （其余） | keep | 高亮上限/粘贴折叠等 | 行为级干净（ed09 的 10 行/1000 字符阈值已钉） | 无 |
| package-tui-engine | （cap 汇总） | keep | 引擎差分渲染合约 | 行为级干净 | 无 |
| package-tui-expandable-output | （cap 汇总） | keep | 展开输出行为 | 行为级干净 | 无 |
| package-tui-interaction-modes | （cap 汇总） | keep | 交互模式 API | 行为级干净 | 无 |
| package-tui-keybindings | （cap 汇总） | keep | 键位解析 | 行为级干净；valid_scope 两文件路径放宽 | 无 |
| package-tui-markdown | （cap 汇总） | keep | Markdown 渲染 | 行为级干净；valid_scope 钉 markdown.rs 放宽 | 无 |
| package-tui-paste-burst | （cap 汇总） | keep | 粘贴突发检测 | 行为级干净（tt04 已钉边界） | 无 |
| package-tui-terminal-protocol | tp01 | rewrite-product | 调 set_kitty_protocol_active(true)（keys.rs 全局） | 探测行为（CSI 序列、有界窗口、回退 modifyOtherKeys）是可观察协议行为，保留；删 `keys.rs 全局` 路径 | 无 |
| package-tui-theme | （cap 汇总） | keep | 主题 token | 行为级干净 | 无 |
| package-tui-tree-selector | （cap 汇总） | keep | 树选择器 | 行为级干净 | 无 |
| protocol-app | （cap 汇总） | keep | 线协议词汇与 serde 形态（pa-m1 已钉 tagged 形态） | 数据契约干净（ip9 的 dispatch 归属是组合根 seam，可留） | 无 |
| runtime-config | rc1 | rewrite-product | ConfigValueResolver MUST 位于 infra/config/value.rs；agent/ MUST NOT 含 config_value.rs | 「ConfigValueResolver（或等价）MUST 收敛于 infra 配置边界且零内部依赖；agent 层 MUST NOT 含重复解析实现」——删路径与防复活半句 | 无 |
| runtime-config | （其余） | keep | 三层合并/插值/默认值 | 行为级干净（tb4 已由单测覆盖声明） | 无 |
| runtime-model-registry | （cap 汇总） | keep | 模型注册/选择 | 行为级干净；valid_scope 钉 bootstrap.rs 放宽 | 无 |
| runtime-resource-discovery | （cap 汇总） | keep | 资源发现 | 行为级干净 | 无 |
| server-core | （cap 汇总） | keep | REST/WS/单实例锁/反向 RPC | 行为级干净（sr-driver1/sr-dispatch1 是跨面同源 seam，可留） | 无 |
| test-bdd | tb4 | rewrite-product | 配置加载行为 MUST 由 src/infra/config 与 src/infra/settings 的单元测试覆盖 | 「配置加载行为（三层合并、解析、插值、默认值）MUST 由单测覆盖，MUST NOT 保留无 step 实现的孤儿 feature」——删路径钉 | 无 |
| test-bdd | （其余） | keep | Partitioned live feature 链路 | test-* 域交付物契约，干净 | 无 |
| test-fake-provider / test-hooks-wiring / test-infra / test-provider-integration / test-qa-gate | （cap 汇总） | keep | 测试基础设施契约 | test-* 域可 keep（qg01/qg04/qg06 的脚本名与闸顺序是 qa 契约本体） | 无 |
| test-standards | ts03/ts04 | move-to-AGENTS | queue.rs/retry.rs/commands.rs/config_value.rs 与 model_manager.rs 等 MUST 有 #[cfg(test)] | 文件级覆盖清单 = 代码组织（且已随模块改名漂移：config_value 已迁 infra）；迁 src/AGENTS.md 或改「对应纯逻辑/子组件 MUST 有单测（清单随代码维护）」 | 无 |
| test-standards | ts01/ts02 | keep | BDD 与单测分界 | 测试政策可留（ts02 的「禁止 domain/ 顶栏挂载」半句删） | 无 |
| user-experience | ux1–ux4 | rewrite-product | System MUST 提供 get_provider_login_help / format_no_models_available_message… | 「System MUST 提供登录引导消息（引用 /login 与文档路径）、无模型/未选模型/无 key 提示（含 provider 名）」——删 4 个函数名钉（改「或等价入口」） | 无 |
| app-tui-host（feature 文件） | ath10 场景 | rewrite-product | 场景步骤「假如 打开 src/app/tui/AGENTS.md」 | 随 ath10 迁出，场景删除或改为行为断言（无进度表/含布局表等内容要求即 AGENTS 自审） | 无 |

## 最高摩擦 / 建议先改（≤10 行）

1. **ath12 + qg06 + tests.rs + check_complexity.py 四处双写复杂度闸** → 合并迁 `src/AGENTS.md` 体量策略，spec 留「入口协调者复杂度受 qa 闸」一句指针（本表 move-to-AGENTS 行）。
2. **layer-architecture 26 条几乎全钉路径** → la1/la2/la4/la6/la7/la15/la17/la19/la20 一次性改写为层名/职责叙述；la9/la21/la-dispatch-consume 迁 AGENTS；la14 删。
3. **atc4/atc26 + adp* 整 cap 的文档路径 SSOT** → 改写为「单一书面视觉 SSOT + 反冗余」两句（atc4/atc26），adp1 留生成物不变量，adp10 删（Next-wave 名单已全部落地）。
4. **tt02 行号引用已过期**（178-184 现为差分渲染测试，变通已被 tests/support TuiTestHarness 取代）→ 删变通句。
5. **BDD 元闸 req 族**（a10/s8/s17/t13/t16/c6/ar-pilot + 各 req 句尾「MUST 有可执行 BDD 场景」）→ 一次性删（由 test-bdd/CI 管）。
6. **防复活句族**（la14、ce11、r71、rc1、ar13）→ 删（对象均 verified 不存在）。
7. **65 个 valid_scope 文件路径钉** → 放宽为层/包范围（agent 层 + infra 层…），迁不进产品的文件清单去各自 AGENTS。
8. **ath10 整条**（AGENTS.md 应写什么的自指要求）→ 它就是面 AGENTS 内容；feature 里同题场景同步改。
9. **ux1–ux4、ed03、tp01、atc2/atc24、dl1、rc1、ce1、tui4、ast9 等函数名/路径钉** → 按本表改写草稿执行（多为「或等价」化或删名）。
10. **歧义补钉**：ath6「同序同分支」验证口径、atc19「NextTurn 消费」定义、ath24 Loader 帧推进与上区缓存边界、avs1 H1–H9 测试名引用 → 改写时各补一句定义。
