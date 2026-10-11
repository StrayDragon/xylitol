# language: zh-CN
# capability: app-tui-host
# purpose: 产品 TUI 宿主：host 驱动引擎、终端生命周期、即时日志、尺寸降级，以及合成垂直切片 / 产品 PTY smoke 验收。
# scope: src/app/tui/, tests/tui_e2e/

功能: app-tui-host

  @req:r1240
  规则: host-driven-engine
    产品 TUI 须以 host 驱动引擎（事件分发、按需渲染、idle 心跳）驱动 packages/xylitol-tui；产品路径 不得进入引擎的阻塞主循环（包 demo 专用启动 API 勿用于生产面）。
    # verified-by: src/app/tui/mod.rs
    场景: host-pump-is-the-single-engine-entry
      当 以主机泵在 idle 提交 bang 命令
      那么 execute_bash 收到命令体且未调用 run

  @req:r1248
  规则: terminal-restore
    产品 TUI 须在正常退出、错误退出与 panic 路径恢复终端（关闭 raw mode 等），不得把用户终端留在损坏状态；须安装 panic hook（仅靠 Drop 不足）。

    场景: terminal-restore-on-exit
      当 以主机泵在 idle 提交 "/exit"
      那么 会话请求退出且终端已恢复
    # verified-by: src/app/tui/terminal_guard.rs
  @req:r1259
  规则: infra-logging-default
    debug 构建下产品 TUI 须默认启用即时文件日志（人类与 agent 可用 tail -f 观察）；release 构建 须默认关闭，仍可通过 XYLITOL_DEBUG 或 RUST_LOG 显式打开。日志路径与查看方式由面 AGENTS 文档写明。
    # verified-by: src/app/cli/logging.rs
    场景: debug-build-enables-file-logging-by-default
      当 以临时目录请求即时文件日志
      那么 debug 构建默认安装文件日志且日志文件已落盘

  @req:r1270
  规则: resize-and-min-size
    终端 resize 须触发重绘且不得丢失输入焦点语义；当引擎因极端尺寸无法安全渲染时 须干净恢复终端并退出进程，不得卡死终端模拟器。宽度小于 40 或高度小于 6 时 须显示友好提示而非 panic。

    场景: min-size-friendly-hint
      假如 以小于最小尺寸的终端挂载主机泵
      当 渲染当前主机帧
      那么 显示友好的最小尺寸提示而非 panic
    # verified-by: src/app/tui/tests.rs
  @req:r1277
  规则: host-harness-testable
    产品 TUI host 须将事件推进与终端 I/O 解耦（可注入事件与终端），使 min-size 提示、事件分发与无阻塞主循环路径可在无真 TTY 的 harness 中验证。
    # verified-by: src/app/tui/harness.rs
    场景: harness-verifies-without-real-tty
      假如 以小于最小尺寸的终端挂载主机泵
      当 渲染当前主机帧
      那么 显示友好的最小尺寸提示而非 panic

  @req:r1278
  规则: shared-effect-pump
    产品 TUI 须经单一事件泵（或等价共享入口）消费 HostSession pending 并调用 Driver/dispatch；合成 harness 泵 须复用同一入口或与其同序同分支（同序验证口径：harness 断言与生产循环对同一 pending 集合按相同分支次序产生相同副作用调用序列），不得维护与生产循环分叉的第二份 slash/bash/steer 副作用 match。
    # verified-by: src/app/tui/harness.rs
    场景: harness-pump-shares-production-effect-branches
      当 以主机泵在 idle 提交斜杠 session-tree
      那么 会话树打开且未作为 prompt 调用 run

  @req:r1279
  规则: single-mux-loop
    产品 TUI host 须以单一扇入模型同时等待终端输入、idle tick、可选 agent EventStream、可选 bang 完成与 bang 输出事件：生产主环与 harness 须调用同一共享 bang/主环事件臂（或单一 select 拓扑）；不得保留与共享入口分叉的第三套 bang Esc select；bang 进行中 Esc 须仍可达 Driver（或等价）abort 并走取消说明；bash 输出事件（session/bash_output）到达 须仅标记 dirty 并在 Tick 或 BangDone 时按需渲染，不得每事件强制全屏重绘；进行中的 bang 须仍可 poll agent EventStream（不得 bang-only 饿死 agent）。

    场景: bang-esc-single-loop
      假如 交互 bang 正在运行且驱动为 ScriptedDriver
      当 以主机泵开启忙碌流并按下 Esc 后收流关闭并渲染
      那么 abort 计一次且未退出且回到可输入 idle

    场景: bang-polls-agent-stream
      假如 交互 bang 正在运行且驱动为 ScriptedDriver
      当 以主机泵提交挂起 bang 并注入事件后按 Esc
      那么 事件被消费且中止计一次且未退出
    # verified-by: src/app/tui/mod.rs
  @req:r1241
  规则: agents-layout-map
    产品 TUI 面 AGENTS 文档 须提供本地布局地图（目录/文件职责、协调者与可下沉模块边界、硬约束与验证命令指针）；不得把进度板或易腐清单写入 AGENTS。
    # verified-by: src/app/tui/AGENTS.md
    场景: tui-agents-doc-carries-layout-map
      当 读取产品 TUI 面 AGENTS 文档
      那么 文档含本地布局地图与验证命令指针且无进度板

  @req:r1243
  规则: host-module-boundaries
    产品 TUI 的组件与 layout 不得直接调用 Driver；HostSession 协调逻辑 须位于可单测切片并经单一事件泵驱动。
    # verified-by: src/AGENTS.md
    场景: effects-go-through-the-single-pump
      当 以主机泵在 idle 提交 "/reload"
      那么 重载经共享缝完成且尾插 Reload 步进汇总

  @req:r1244
  规则: session-read-errors-surfaced
    产品 TUI 在 travel/fork/label 等路径调用 Command::GetMessages（或等价）失败时 须向用户展示 system/error note，不得以 unwrap_or_default 静默得到空 transcript 并当作成功。

    场景: get-messages-failure-surfaced
      假如 驱动对消息快照返回失败
      当 以主机泵注入可恢复会话列表后提交 "/session-resume"
      当 以主机泵在面板中确认选定
      那么 写入错误提示而非静默空 transcript
  @req:r1245
  规则: session-list-seam
    产品 TUI 获取可 resume 会话列表时 须经 Driver（或 app::core 公开 seam）list_sessions 或等价 API，行 须能支撑 scope（cwd）、预览、parent、mtime/age 与可选 path 展示；不得从 app/tui 直接 import infra::session / 直接读 sessions 目录；switch 成功后 须重建 transcript 并清除与旧 session 绑定的树槽/pending UI 态。
    # verified-by: llmanspec/specs/app-tui-host/app-tui-host.feature
    场景: resume-list-rows-via-public-seam
      当 以主机泵注入可恢复会话列表后提交 "/session-resume"
      那么 Resume 面板槽打开且经驱动列举可恢复会话

  @req:r1246
  规则: session-lifecycle-seam
    产品 TUI 创建空会话与读写会话显示名时 须经 Command::NewSession / GetSessionName / SetSessionName（或 app::core 公开等价 seam），不得从 app/tui 直接 import infra::session 或直接写 sessions 目录；set_session_name 须将 CR/LF 规范为空格并 trim；new_session 成功后 须清空与旧 session 绑定的 transcript/树槽；session-clone 须仅经既有 Command::Fork(At)+Command::SwitchSession 路径，不得平行 fork 实现。

    场景: session-name-normalizes-crlf
      当 以主机泵在 idle 提交带换行的 "/session-name 甲 乙"
      那么 会话名写入时换行规范为空格
  @req:r1247
  规则: session-resume-manage-seam
    产品 TUI Resume 面板内 rename/delete 须经 Command::SetSessionName / DeleteSession（或 app::core 公开等价 seam）；删除当前活跃 session 须拒绝并提示且 不得调用删除；删除 须经显式确认（确认前 不得删盘）；scope=Current 须仅展示 cwd 匹配会话，scope=All 须展示 Command::ListSessions 或等价列举的全部可 resume 会话；TUI 不得直删会话文件。

    场景: resume-delete-current-refused
      当 以主机泵注入含当前会话的可恢复列表后提交 "/session-resume"
      当 以主机泵在面板中按删除
      那么 提示无法删除当前会话且未调用删除

    场景: resume-delete-requires-confirm
      当 以主机泵注入可恢复会话列表后提交 "/session-resume"
      当 以主机泵在面板中按删除再确认
      那么 删除恰经驱动调用一次
    # verified-by: llmanspec/specs/app-tui-host/app-tui-host.feature
  @req:r1249
  规则: reload-orchestrates-foundations
    产品 host/effects 在处理 /reload 时 须经 Driver/composition 缝调用已归档的 reload_skills、MCP reload、keybindings reload、theme/context reload（若已接线），须在 skills 重载后刷新产品 $skill 补全 catalog；不得 reach agent/infra 内部绕过 seam；任一步失败 须记录诊断并继续其余步骤（部分成功）。

    场景: reload-orchestrates-foundations-report
      当 以主机泵在 idle 提交 "/reload"
      那么 重载经共享缝完成且尾插 Reload 步进汇总
  @req:r1251
  规则: thinking-level-silent-commit
    产品 host 经 /model 槽或有参 /model 提交 model/thinking、或经 /theme 应用主题时 须经共享执行器更新 selected；成功路径 须仅更新固定区（footer、边框、主题），不得向 live scrollback / transcript 追加 model → … / thinking-border → … / theme → … 类滚动提示确认块；失败或诊断 MAY 写滚动提示。agent-busy 时 footer 须立即反映 selected（run 绑定语义；产品面无换模预告）。模型切换后 须从 Driver 重同步 UI；组件 不得直接 reach ModelManager。

    场景: silent-commit-model-and-theme-fixed-zone-only
      当 以主机泵在 idle 提交 "/model test-model"
      那么 有参直设经模型执行器更新且仅更新固定区
      当 提交 "/theme dark"
      那么 主题直接应用且槽关闭
  @req:r1252
  规则: loaded-resources-via-driver
    产品 host 须经 Driver 只读缝获取 skills 名与 MCP 连接摘要（含连接进行中进度或等价 phase）以填充 loaded-resources；启动过程中 须能在 MCP 未全部完成时刷新该槽；启动完成与 /reload 成功后 须再刷新；新会话与 CLI `--session` resume 进入 TUI 时 不得等待全部 MCP 连接完成才渲染面或投影历史；MCP 未结算前 不得因 connecting 拒绝用户键入、提交普通 agent prompt、bang 或 slash（含 `/reload`）（agent busy 既有闸除外），须允许滚历史；首次 generate 的 MCP 门闸/定稿语义见 infra-mcp mcp8（可提交，生成可等待）。面内 `/session-resume` 切会话 不得为切换而阻塞重连 MCP。idle `/reload` 须可触发工具重定稿（mcp8）；busy 时 /reload 拒绝语义保持 ath20/既有行为。不得从 app/tui reach infra::mcp 或 skills 目录。

    场景: connecting-mcp-does-not-block-input
      当 以主机泵注入 connecting 资源快照后提交 bang 命令 "!echo hi"
      那么 键入与提交未被拒且 bash 收到命令体
  @req:r1253
  规则: tick-gated-local-paint
    产品 host 在 Ready 态处理 Tick 时：须先推进 idle 心跳；仅当 idle 心跳报告 dirty 或 host 已置 paint_dirty（含 bang 输出事件追加）时才请求渲染；idle 且无 paint_dirty 时 不得仅为 Tick 整帧重绘。UiRoot 对 loaded-resources+scrollback+queue 须在仅 status Loader 动画帧推进（含 Loader 帧换与纯 status 短词变化）时复用上区行缓存；仅当 entries、streaming tails、queue strip、fold、theme、loaded-resources 或宽度变化时 须失效该上区缓存。

    场景: idle-tick-no-full-repaint
      假如 以主机泵在空闲态渲染基线帧
      当 空闲态推进一次 Tick
      那么 未触发整帧重绘
  @req:r1254
  规则: scrollback-entry-paint-cache
    产品 live scrollback 绘制 须对已提交 UiEntry 使用按条目指纹的行缓存：在 width 与 fold 不变时，仅 streaming tails 或发生变化的条目 须触发该条目（及必要时其后条目）重绘；不得因单次 TextDelta/ThinkingDelta 而对全部历史 Assistant 条目重新 Markdown 解析。须在无真 TTY harness 中可测（重绘/miss 计数上界）；可选 PTY/tmux e2e 冒烟 须在较大 scrollback 下仍能完成一轮并干净退出，不得以 OS CPU% 作为硬闸。
    # verified-by: src/app/tui/widgets/scrollback/mod.rs
    场景: committed-entries-reuse-row-cache
      假如 构造含两条已提交助手条目的 UI 并渲染基线帧
      当 仅推进流式尾标再渲染一次并读取已提交条目重绘计数
      那么 已提交条目在第二次渲染零重绘

  @req:r1255
  规则: streaming-assistant-incremental-paint
    产品对 streaming_assistant 的 live Markdown 绘制 须在 width/theme 不变且文本仅后缀增长时，复用已稳定 Markdown 前缀（段落边界，且不切开未闭合代码围栏）的已渲染行，仅对不稳定后缀（含流式 … 尾标）重新 Markdown 解析；不得在每次 TextDelta 上都对整段 streaming 缓冲做全量解析（当已存在可复用稳定前缀时）。本要求 不得改变 tool/bash/write/diff 的 Ctrl+O 视口折叠/展开或硬截断禁展开语义（att14/att16）。须在无真 TTY harness 中可测（全量解析计数上界 + 与全量解析行一致）。
    # verified-by: src/app/tui/widgets/scrollback/mod.rs
    场景: streaming-suffix-growth-limits-full-parses
      假如 构造含稳定 Markdown 前缀的流式助手 UI
      当 仅以后缀增长连续渲染四十次并读取全量解析计数
      那么 全量 Markdown 解析次数远小于渲染次数
  @req:r1937
  规则: streaming-assistant-partial-fence-height
    产品对 streaming_assistant 的 live Markdown 绘制 须使用与包 Markdown 同一把半截闭合围栏尺子：开围栏后代码正文再跟不完整 closer、随后补全 closer 时，该流式块可见行数 须不变。不得另写第二套围栏修剪。本要求不改变 r1255 的稳定前缀复用与全量解析上界，不得改变 tool/bash/write/diff 视口折叠语义。须在无真 TTY harness 中可测。
    # verified-by: src/app/tui/widgets/scrollback/paint.rs
    场景: streaming-assistant-partial-fence-height-headless
      假如 构造含开围栏与代码正文的流式助手 UI
      当 追加半截闭合围栏再渲染并补全闭合围栏再渲染
      那么 两次流式块可见行数相同

  @req:r1256
  规则: mcp-discovery-surface
    产品 host 须将 MCP 发现主路径接到 `/mcp` SelectList（atm17）：refresh_loaded_resources（或等价）须更新可供 `/mcp` 同步挂载的缓存快照；Driver 只读缝 须能支撑列表行（每 server 连接态 + tools armed）与汇总。MCP 仍 connecting、或工具表尚未 FROZEN 且 bootstrap 未完成（含 resume 后 Settling、旧工具仍 armed）、或 Connected 尚未 armed 且未冻表时，MAY 在 busy 下轮预告位或 idle status 显示短 cue；已提交门闸 status lead=`Assembling` 时右侧 须可与该 cue 并存。文案 须固定为 `mcp pending (see /mcp)` 且右对齐（不得分数计数，不得枚举 server id）。Failed 或（bootstrap 已完成且已 FROZEN）后的未武装 不得单独拖住短 cue（细节进 `/mcp`）。connecting 收口 / 冻表后短 cue 须收起（除非仍有 Connecting）。头卡 loaded-resources mcp 行仍可作启动摘要，但 不得作为长对话唯一发现面。不得因开 `/mcp` abort agent；不得从 app/tui reach infra::mcp。

    场景: mcp-pending-cue-in-status
      当 以主机泵注入 connecting 且未冻表的资源快照后渲染当前主机帧
      那么 status 短 cue 为固定文案 mcp pending (see /mcp) 且不枚举 server id
  @req:r1257
  规则: reload-in-progress-ux
    产品 TUI 在 idle 启动无参 /reload 后、重载未完成前 须：以独立 reload 进行中态（非 agent run_active、非 bang busy）驱动 status lead 为 spinner+`Reloading`；host/effects 不得因 await 重载而阻塞终端 tick 与键入处理。进行中 须允许打字与 Ctrl+G 外编；Enter 提交普通上行 / slash（含二次 /reload）/ bang 须拒绝且经通知条 body 恰好为 `reloading — wait`（可见 `Error: reloading — wait`），不得调用第二次 runtime reload 或入 steer。无 overlay 时 Esc 与 Ctrl+C 须请求协作取消重载（非退出）；有 overlay 时 须先关槽。取消收口 须恢复一致工具/MCP 快照（旧 manager 保留至新装好或取消恢复）、put-back reload 句柄、解除进行中态，并以滚动提示 `Reload cancelled:`（含已完成步进摘要）+ 通知条 body `reload cancelled` 说明；已写入的 skills/context 不得要求事务回滚。失败（含墙钟超时诊断）须以既有 `Reload:` 步进报告 + 通知条 body `reload failed — see report` 说明并解除进行中态。成功路径仍尾插既有 `Reload:` 汇总；不得清空 transcript/session 历史或 editor 草稿。agent/bang 真 busy 时 /reload 拒绝保持 ath20/atm12。墙钟对齐既有 MCP 单 server 与首 turn 门闸常量。验证 须含可注入慢/失败/取消的 harness。

    场景: reload-soft-gate-enter-toast
      当 以主机泵挂起重载进行中提交上行并取消收尾
      那么 提交被软闸拒绝且草稿保留且未发第二次运行时重载
  @req:r1281
  规则: synthetic-harness-one-round
    产品 TUI 须提供可在无真 TTY/无真 LLM 下运行的合成验收：ScriptedDriver（或等价）记录 run/steer/follow_up/abort/clear_queue，并能将预置 XyEvent 流回流 HostSession。须覆盖全链场景清单：提交→流式/工具→steer/follow-up queue strip→abort 后再提交→/exit 触发 finish（按当前交互模式分发 teardown；默认 ApplicationOwned）并 stop（场景清单随 harness 命名漂移，以全链覆盖语义为准）；不得把活树、bash、compaction UI 纳入本切片。

    场景: synthetic-harness-full-round-chain
      当 以合成验收链驱动一整轮含工具与 abort 的会话并 /exit
      那么 提交流式工具 steer abort 再提交按序生效且会话请求退出
  @req:r1282
  规则: product-pty-fake-smoke
    仓库 须含产品二进制的 PTY 冒烟（#[ignore]，经 just test-tui-e2e-pty 可跑）：在临时 Fake 模型配置与 --trust 下启动 TUI，提交用户消息后屏上 须出现 Fake 默认文案 Hello from fake provider，输入 /exit 后进程 须退出。默认 just qa 不得强制该用例；不得要求跨进程 Fake 脚本化，不得在本用例断言 steer/工具 strip。
    # verified-by: justfile
    场景: pty-smoke-registered-and-opt-in
      当 读取 PTY 冒烟登记
      那么 存在 just test-tui-e2e-pty 且冒烟为 ignore 不进默认门禁

  @req:r1258
  规则: mouse-input-opt-in-no-moved-paint
    产品 host 扇入 须能将 crossterm Event::Mouse 映射为 HostEvent::Input(InputEvent::Mouse)。ApplicationOwned 指针探针未武装时 Moved MAY 在映射前丢弃。探针武装后 Moved 须送达引擎；无命中或悬停目标未变 不得因此整帧重绘。Ready 态处理 Mouse 时：无 UI dirty / 未消费态变 不得 request_render（含无命中 Moved）；消费态变（如 ApplicationOwned 拖选或悬停目标切换）MAY request_render。产品 不得经 TerminalGuard / `XYLITOL_TUI_MOUSE` 在进 TTY 时默认开 mouse capture；ApplicationOwned 会话 begin 后的 capture 见 ath30。teardown 须 Disable mouse capture（尽探针所能断言 mouse mode 不残留）。须提供 #[ignore] PTY 用例（经 just test-tui-e2e-pty 可跑）；默认 just qa 不得强制该用例。折叠点击语义不在本要求范围。

    场景: mouse-moved-no-repaint
      假如 以主机泵在空闲态渲染基线帧
      当 注入无态变的鼠标移动
      那么 未触发整帧重绘

    场景: mouse-moved-armed-probe-delivers-headless
      假如 以主机泵在空闲态渲染基线帧
      当 指针探针已武装并注入落在折叠块头的鼠标移动
      那么 未触发整帧重绘且悬停目标已送达
    # verified-by: src/app/tui/host/mod.rs
  @req:r1260
  规则: interaction-mode-application-owned-default
    产品 TUI 须在 host **启动构造时**绑定 xylitol-tui 交互模式为 ApplicationOwned（应用自管视口 / alt-screen）。缺省/未配置 须为 ApplicationOwned。一次会话 须只有一个主模式；不得在会话运行中热切模式（改模式 = 结束进程或新建 HostSession，不是 mid-loop 换栈 API）。不得把 XYLITOL_TUI_MOUSE 环境变量当作产品模式开关；不得提供面向用户的 Inline/ApplicationOwned 切换设置。产品 须将下缘固定区（至少 status/editor/footer 所占行）登记为 dock，使包级选区排除输入面；teardown 须不残留 mouse capture / alt-buffer。退出 ApplicationOwned 时产品 须依赖库 finish dump（或等价）使主屏 scrollback 仍可读会话内容；本要求不要求向用户暴露 dump opt-out。折叠点击语义不在本要求范围（见后续 fold 族 change）。库仍可暴露 Inline 构造入口供 lab/demo；产品默认路径 不得使用 Inline。
    # verified-by: src/app/tui/host/mod.rs
    场景: product-ui-binds-application-owned-at-construction
      当 以产品 UI 构造主机会话并查询交互模式
      那么 产品交互模式为 ApplicationOwned

  @req:r1261
  规则: mode-b-copy-notice-fixed-zone
    产品 TUI 在 ApplicationOwned 会话下，当库发出松手复制成功 copy-notice（ptim15）时 须展示短时用户可见提醒（TTL 约 1.5–3s 后自动消失）。落点 须为右上角客户端通知栈（见 r1930），且 须钉在已裁切视口而非滚动内容顶；不得写入 transcript / ScrollNotice；不得使用带 `Error: ` 前缀的拒闸通知条冒充成功确认；不得再占用 status 旁下缘行。折叠点击不在范围。

    场景: copy-notice-fixed-zone-cue
      当 臂装复制成功提示后渲染主机帧
      那么 右上角通知栈含 Copied 短提示且不以 Error 前缀冒充
  @req:r1262
  规则: enter-follows-transcript-bottom
    产品 TUI 在 ApplicationOwned 的 Ready 主输入面收到 `tui.input.submit` Enter 时 须立即将 transcript 视口滚到底部并恢复尾插；该行为 须同时适用于非空提交与空输入。空输入 不得因此创建 submit；非 Editor 槽中的 Enter 须保留给该槽自身的确认语义，不得强制滚动 transcript。

    场景: empty-enter-no-submit
      当 空输入按下 Enter
      那么 未创建提交且仍在编辑器槽
  @req:r1263
  规则: fold-pointer-hit
    产品 TUI 在 ApplicationOwned 会话下 须将 live scrollback 折叠命中接到引擎指针命中管线：空选左键命中已登记折叠区时 须清除 transcript 选区且不启拖选，并触发该目标的定点 toggle；未命中时 须保持既有选区/dock/Editor 路径。有选区或拖选进行中 须不消费折叠命中、不得因此复制。产品 不得经 XYLITOL_TUI_MOUSE 作为折叠开关。验证 须含无真 TTY harness（合成 Mouse）。

    场景: fold-pointer-click-toggles-block
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      并且 左键单击折叠命中表中的簇头三角列
      那么 仅该簇被定点展开且内层块行可见
  @req:r1264
  规则: attach-tick-no-blocking-rpc
    产品 TUI attach 路径的 host 循环在 idle/busy tick 与 drain_pending 中 不得同步阻塞等待 Host unary（含 get_state / current_model / MCP settle）。status spinner 须在 Host 慢连接或写者 unary 进行中仍能换帧；键入 须仍可进入 editor。写者 unary（含 SetModel）不得在返回前等待全部 MCP 连接完成。模型列表打开与选定 须在 MCP 未结算时仍可操作（选定可走 NextTurn）。
    # verified-by: src/app/tui/host/mod.rs
    场景: attach-drain-not-blocked-by-pending-unary
      假如 以注入短常量的 mock HostClient 完成 attach
      当 在 idle 排空下行事件
      那么 排空不被未决 unary 阻塞且返回可读投影批

  @req:r1265
  规则: attach-mux-session-lifetime
    产品 TUI attach 须在进入 host 循环前完成握手并订阅该 session 的 mux（启动即订，不等第一次 prompt）。单次 AgentEnd 不得拆掉该订阅。WS 断开后 须用 last_seq 再次 subscribe 续传；收到 session/resync_required 后 须按 server-core 再订并重建可见 transcript，不得只留一行错误后放弃事件流。退出 TUI 才关闭 mux。
    # verified-by: llmanspec/specs/app-tui-host/app-tui-host.feature
    场景: agent-end-keeps-mux-subscription
      假如 客户端已订阅会话 s-sub
      当 会话回合以 AgentEnd 结束后又追加新事件
      那么 订阅仍存活且新事件继续送达

  @req:r1266
  规则: attach-restore-projection
    产品 TUI attach 恢复（CLI --session 启动与空闲 Resume 切换）须以 Host 消息快照一次重建 transcript：完整历史一次可见、会话 Idle、无假 spinner、无逐条回放；恢复窗内的冷订实况磁带（AgentStart / TextDelta 等回合磁带）不得渲染进 transcript；不得以 loading 态掩盖回放。
    # verified-by: llmanspec/specs/app-tui-host/app-tui-host.feature
    场景: cold-restore-single-snapshot
      假如 会话 s0 已有 3 条历史条目且客户端无有效 last_seq
      当 冷订客户端调用消息快照
      那么 快照一次返回全部 3 条条目

    场景: cold-restore-ignores-live-tape
      假如 冷订客户端处于恢复窗内且 journal 含实况磁带事件
      当 调用消息快照
      那么 快照内容与磁带回放无关

  @req:r1267
  规则: attach-reload-cooperative-cancel
    产品 TUI attach 下 `/reload` 进行中用户触发取消（interrupt/clear 键位）时，client 须将取消意图经 Host 转达（合作取消该次进程级 reload），不得继续等待原 reload 完成；取消后 reload 界面 须以已取消收尾，Host 须停止后续重装步骤。正常完成路径行为保持不变。
    # verified-by: llmanspec/specs/app-tui-host/app-tui-host.feature
    场景: attach-reload-cancel-stops-later-steps
      当 以主机泵开启重载并输入草稿后按 Enter
      那么 通知条为 reloading 且草稿保留且未新增滚动提示
      当 经输入流注入 Esc 取消挂起重载
      那么 重载结束且通告为已取消

  @req:r1268
  规则: attach-fixed-zone-downlink-driven
    产品 TUI attach 下 MCP/skills 头卡的连接态刷新 须由 Host 的 `session/resources` 下行驱动：帧到达置脏后由 tick 读本地缓存刷新；不得以周期性 unary 轮询同一刷新。首帧到达前的初始快照 MAY 经一次性 loaded_resources unary 获取；启动与 /reload 后的既有刷新语义（ath23）保持。
    # verified-by: llmanspec/specs/app-tui-host/app-tui-host.feature
    场景: resource-header-refreshed-from-snapshot
      当 以含 skills 与 connecting MCP 的资源快照渲染头部卡
      那么 卡含 skills 行与 mcp 行且展示连接进度

  @req:r1269
  规则: unary-request-bounded
    产品 TUI 对 Host 的每笔 unary 请求 须有请求级等待界；已知长操作（如 reload）须分级放宽且各有界；半开连接下 须以超时错误呈现而非无限挂起。
    # verified-by: src/app/core/host_client/http_ws.rs
    场景: subscription-idle-wait-is-bounded
      假如 以短空闲超时的产品订阅客户端已升级 WS /rpc
      当 超过空闲超时仍无入站帧
      那么 客户端判定半开并结束该订阅

  @req:r1271
  规则: mux-halfopen-detect
    产品订阅连接（WS /rpc）须周期性探测连接活性（如 keepalive ping）；连续探测周期无任何入站帧时 须判定半开并走既有重订/resync 路径，不得静默停摆。

    场景: mux-halfopen-idle-detect
      假如 以短空闲超时的产品订阅客户端已升级 WS /rpc
      当 超过空闲超时仍无入站帧
      那么 客户端判定半开并结束该订阅
  @req:r1272
  规则: attach-reconnect-backoff-generation
    产品 TUI attach 的订阅重连循环 须指数退避，且单次连接存活达到阈值后才 须归零退避：存活不足阈值的反复闪断 须按持续故障逐次升级，不得以固定高频重试冲击 Host。重订/重连循环 须携带代际计数，旧代连接的迟到帧 不得进入新代投影。

    场景: reconnect-backoff-escalation
      假如 以注入短常量的 mock HostClient 驱动 attach 重连
      当 连续建立存活不足归零阈值即断开的连接
      那么 重试间隔 逐次翻倍升级
      当 某次连接存活达到归零阈值后断开
      那么 下次重试间隔 回落到起点

    场景: reconnect-stale-generation-dropped
      假如 已有一条订阅中的 mock mux 连接
      当 触发重订或换会话产生新代循环后旧代连接迟到推入事件
      那么 该迟到事件 不出现在新代的 drain 结果中
  @req:r1273
  规则: attach-reconnect-grace-ux
    产品 TUI attach 下初始连接与重连 须各有宽限期：宽限内 不得因断线/重试打扰信息面（transcript 与状态条零输出）；超宽限 须以通知条（toast notice）提示断线重连中，恢复成功 须以通知条收尾并清除断线态；重连窗内 不得向 transcript 推错误行。
    # verified-by: llmanspec/specs/app-tui-host/app-tui-host.feature
    场景: reconnect-grace-then-toast
      假如 真进程 serve 已启动且 attach 客户端已订阅
      当 server 的 mux 连接被断开且期间 journal 新增事件
      那么 客户端 在宽限内不上屏断线错误并自动重连

  @req:r1274
  规则: attach-coalesce-downlink
    产品订阅下行事件 须在短窗口内攒批合帧：同窗口多条事件 须合并为一次 UI 投影批消费，不得逐事件触发投影。

    场景: attach-coalesce-burst-single-projection
      假如 合帧窗口开启
      当 窗口内连续到达多条下行事件
      那么 客户端 以一次投影批消费且消费批数等于合并后批数而非事件条数
  @req:r1275
  规则: attach-hello-handshake
    产品 TUI attach 后 须经已登记方法（host.describe 或语义等价）的 result 校验协议版本；版本不符 须按不可重试故障终止（fatal），不得降级、不得重试风暴。握手版本语义 须单一，不得同时维护特例首帧与方法 result 两套协商。连接失败（含订阅失败）须进入重连判定。

    场景: attach-hello-mismatch-fatal
      假如 mock HostClient 握手返回不符协议版本
      当 attach 客户端完成握手校验
      那么 driver 报版本错误并终止该代循环且 不进入重试循环

    场景: real-kill-reconnect-journal-resume
      假如 真进程 serve 已启动且 attach 客户端已订阅
      当 server 的 mux 连接被断开且期间 journal 新增事件
      那么 客户端 在宽限内不上屏断线错误并自动重连
      并且 重连后 按 last_seq 从 journal 续传缺失事件且 不依赖人工重开
  @req:r1276
  规则: interactive-loop-pumps-inline
    交互 bang 循环 须在既有 tick 节奏内经窄入口（drain_inline_pending）调度执行类为 Inline 的 pending effects（消费 atm18 exec_class），不得阻塞 bang 输出 chunk 的既有转发节奏；执行类为 Exclusive 与 Queued 的 pending 须保持排队至循环归还后由主循环 drain_pending 处理且语义不变；主循环 drain_pending 的全量调度行为 不得改变。

    场景: exclusive-stays-queued
      假如 交互 bang 正在运行
      当 提交 busy-Allow 且执行类为 Queued 的 /session-export
      那么 该命令 在 bang 结束前不执行
      并且 循环归还后 照常执行且 不丢失

  @req:r1953
  规则: session-nav-notice-trailing
    产品 TUI 在空闲会话切换、新建、clone、import 与 CLI restore 成功并重建 transcript 后，须以滚动提示尾插对应导航文案（switched → / new session → / cloned → / imported → / restored →），跟底时 须出现在输入框上方视野。不得写入通知栈或通知条。busy 拒切仍走通知条（r1189）。树 fork 的 forked → 与 travel 的 history @ 已有尾插，须保持、不得改送通知栈。
    场景: session-switch-trails-not-stack
      当 以主机泵完成空闲会话切换并渲染
      那么 滚动提示尾插含 switched 且通知栈不含该文案

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）
