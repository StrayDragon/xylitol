# language: zh-CN
# capability: app-tui-host
# purpose: 产品 TUI 宿主：host 驱动引擎、终端生命周期、即时日志、尺寸降级，以及合成垂直切片 / 产品 PTY smoke 验收。
# scope: src/app/tui/, tests/tui_e2e/

功能: app-tui-host

  @req:ath1 @human
  场景: host-driven-engine
    - 产品 TUI MUST 以 host 驱动引擎（事件分发、按需渲染、idle 心跳）驱动 packages/xylitol-tui；产品路径 MUST NOT 进入引擎的阻塞主循环（包 demo 专用启动 API 勿用于生产面）。

  @req:ath2 @human
  场景: terminal-restore
    - 产品 TUI MUST 在正常退出、错误退出与 panic 路径恢复终端（关闭 raw mode 等），不得把用户终端留在损坏状态；MUST 安装 panic hook（仅靠 Drop 不足）。

  @req:ath3 @human
  场景: infra-logging-default
    - debug 构建下产品 TUI MUST 默认启用即时文件日志（人类与 agent 可用 tail -f 观察）；release 构建 MUST 默认关闭，仍可通过 XYLITOL_DEBUG 或 RUST_LOG 显式打开。日志路径与查看方式由面 AGENTS 文档写明。

  @req:ath4 @human
  场景: resize-and-min-size
    - 终端 resize MUST 触发重绘且不得丢失输入焦点语义；当引擎因极端尺寸无法安全渲染时 MUST 干净恢复终端并退出进程，MUST NOT 卡死终端模拟器。宽度小于 40 或高度小于 6 时 MUST 显示友好提示而非 panic。

  @req:ath5 @human
  场景: host-harness-testable
    - 产品 TUI host MUST 将事件推进与终端 I/O 解耦（可注入事件与终端），使 min-size 提示、事件分发与无阻塞主循环路径可在无真 TTY 的 harness 中验证。

  @req:ath6 @human
  场景: shared-effect-pump
    - 产品 TUI MUST 经单一事件泵（或等价共享入口）消费 HostSession pending 并调用 Driver/dispatch；合成 harness 泵 MUST 复用同一入口或与其同序同分支（同序验证口径：harness 断言与生产循环对同一 pending 集合按相同分支次序产生相同副作用调用序列），MUST NOT 维护与生产循环分叉的第二份 slash/bash/steer 副作用 match。

  @req:ath7 @human
  场景: single-mux-loop
    - 产品 TUI host MUST 以单一扇入模型同时等待终端输入、idle tick、可选 agent EventStream、可选 bang 完成与 bang 输出事件：生产主环与 harness MUST 调用同一共享 bang/主环事件臂（或单一 select 拓扑）；MUST NOT 保留与共享入口分叉的第三套 bang Esc select；bang 进行中 Esc MUST 仍可达 Driver（或等价）abort 并走取消说明；bash 输出事件（session/bash_output）到达 MUST 仅标记 dirty 并在 Tick 或 BangDone 时按需渲染，MUST NOT 每事件强制全屏重绘；进行中的 bang MUST 仍可 poll agent EventStream（不得 bang-only 饿死 agent）。

  @req:ath8 @human
  场景: abort-drops-stream-events
    - 产品 TUI 在用户 Busy Esc 置 pending_abort 的同一同步步进内 MUST 立即臂装 suppress_xy_until_stream_end（或等价门闩），使在随后 drain_pending 调用 note_user_abort / Driver::abort 之前到达的 ThinkingDelta / TextDelta / 会提交正文的 AgentEnd MUST NOT 经 apply_xy_event 复活 busy 或写出 assistant 正文；note_user_abort 仍 MUST 清空 streaming_thinking 与 streaming_assistant；Driver::abort MUST 仍被调用；下一轮 submit MUST 可正常开始。

  @req:ath10 @human
  场景: agents-layout-map
    - 产品 TUI 面 AGENTS 文档 MUST 提供本地布局地图（目录/文件职责、协调者与可下沉模块边界、硬约束与验证命令指针）；MUST NOT 把进度板或易腐清单写入 AGENTS。

  @req:ath11 @human
  场景: abort-suppress-before-drain
    - Busy Esc（非 overlay）MUST 在同一同步步进内臂装 Xy 抑制，MUST NOT 仅依赖下一轮循环顶部的事件泵才开始丢弃该轮流事件；bang Esc 路径 MUST 继续走取消语义（无 suppress_xy），不得与 agent abort 文案混用。

  @req:ath12 @human
  场景: host-module-boundaries
    - 产品 TUI 的组件与 layout MUST NOT 直接调用 Driver；HostSession 协调逻辑 MUST 位于可单测切片并经单一事件泵驱动。

  @req:ath13 @human
  场景: session-read-errors-surfaced
    - 产品 TUI 在 travel/fork/label 等路径调用 Command::GetMessages（或等价）失败时 MUST 向用户展示 system/error note，MUST NOT 以 unwrap_or_default 静默得到空 transcript 并当作成功。

  @req:ath14 @human
  场景: session-list-seam
    - 产品 TUI 获取可 resume 会话列表时 MUST 经 Driver（或 app::core 公开 seam）list_sessions 或等价 API，行 MUST 能支撑 scope（cwd）、预览、parent、mtime/age 与可选 path 展示；MUST NOT 从 app/tui 直接 import infra::session / 直接读 sessions 目录；switch 成功后 MUST 重建 transcript 并清除与旧 session 绑定的树槽/pending UI 态。

  @req:ath15 @human
  场景: session-lifecycle-seam
    - 产品 TUI 创建空会话与读写会话显示名时 MUST 经 Command::NewSession / GetSessionName / SetSessionName（或 app::core 公开等价 seam），MUST NOT 从 app/tui 直接 import infra::session 或直接写 sessions 目录；set_session_name MUST 将 CR/LF 规范为空格并 trim；new_session 成功后 MUST 清空与旧 session 绑定的 transcript/树槽；session-clone MUST 仅经既有 Command::Fork(At)+Command::SwitchSession 路径，不得平行 fork 实现。

  @req:ath16 @human
  场景: session-resume-manage-seam
    - 产品 TUI Resume 面板内 rename/delete MUST 经 Command::SetSessionName / DeleteSession（或 app::core 公开等价 seam）；删除当前活跃 session MUST 拒绝并提示且 MUST NOT 调用删除；删除 MUST 经显式确认（确认前 MUST NOT 删盘）；scope=Current MUST 仅展示 cwd 匹配会话，scope=All MUST 展示 Command::ListSessions 或等价列举的全部可 resume 会话；TUI MUST NOT 直删会话文件。

  @req:ath20 @human
  场景: reload-orchestrates-foundations
    - 产品 host/effects 在处理 /reload 时 MUST 经 Driver/composition 缝调用已归档的 reload_skills、MCP reload、keybindings reload、theme/context reload（若已接线），MUST 在 skills 重载后刷新产品 $skill 补全 catalog；MUST NOT reach agent/infra 内部绕过 seam；任一步失败 MUST 记录诊断并继续其余步骤（部分成功）。

  @req:ath21 @human
  场景: copy-last-via-driver-seam
    - 产品 /history-copy-last MUST 经 Driver::copy_text_to_clipboard（或等价）调用剪贴板，MUST NOT 从 app/tui 直接 reach infra::clipboard；选文 MUST 取已提交 UiEntry::Assistant 自尾向前第一条非空正文，MUST NOT 复制 thinking/tool/system 块或未提交 streaming 半句作为成功路径。

  @req:ath22 @human
  场景: thinking-level-silent-commit
    - 产品 host 经 /model 槽或有参 /model 提交 model/thinking 时 MUST 经 Command::SetThinkingLevel / SetModel 执行器更新 selected；成功路径 MUST 仅更新固定区（footer、边框），MUST NOT 向 live scrollback / transcript 追加 model → … 或 thinking-border → … 类滚动提示确认块；失败或诊断 MAY 写滚动提示。agent-busy 时 footer MUST 立即反映 selected（run 绑定语义；产品面无换模预告）。模型切换后 MUST 从 Driver 重同步 UI；组件 MUST NOT 直接 reach ModelManager。

  @req:ath23 @human
  场景: loaded-resources-via-driver
    - 产品 host MUST 经 Driver 只读缝获取 skills 名与 MCP 连接摘要（含连接进行中进度或等价 phase）以填充 loaded-resources；启动过程中 MUST 能在 MCP 未全部完成时刷新该槽；启动完成与 /reload 成功后 MUST 再刷新；新会话与 CLI `--session` resume 进入 TUI 时 MUST NOT 等待全部 MCP 连接完成才渲染面或投影历史；MCP 未结算前 MUST NOT 因 connecting 拒绝用户键入、提交普通 agent prompt、bang 或 slash（含 `/reload`）（agent busy 既有闸除外），MUST 允许滚历史；首次 generate 的 MCP 门闸/定稿语义见 infra-mcp mcp8（可提交，生成可等待）。面内 `/session-resume` 切会话 MUST NOT 为切换而阻塞重连 MCP。idle `/reload` MUST 可触发工具重定稿（mcp8）；busy 时 /reload 拒绝语义保持 ath20/既有行为。MUST NOT 从 app/tui reach infra::mcp 或 skills 目录。

  @req:ath24 @human
  场景: tick-gated-local-paint
    - 产品 host 在 Ready 态处理 Tick 时：MUST 先推进 idle 心跳；仅当 idle 心跳报告 dirty 或 host 已置 paint_dirty（含 bang 输出事件追加）时才请求渲染；idle 且无 paint_dirty 时 MUST NOT 仅为 Tick 整帧重绘。UiRoot 对 loaded-resources+scrollback+queue MUST 在仅 status Loader 动画帧推进（含 Loader 帧换与纯 status 短词变化）时复用上区行缓存；仅当 entries、streaming tails、queue strip、fold、theme、loaded-resources 或宽度变化时 MUST 失效该上区缓存。

  @req:ath25 @human
  场景: scrollback-entry-paint-cache
    - 产品 live scrollback 绘制 MUST 对已提交 UiEntry 使用按条目指纹的行缓存：在 width 与 fold 不变时，仅 streaming tails 或发生变化的条目 MUST 触发该条目（及必要时其后条目）重绘；MUST NOT 因单次 TextDelta/ThinkingDelta 而对全部历史 Assistant 条目重新 Markdown 解析。MUST 在无真 TTY harness 中可测（重绘/miss 计数上界）；可选 PTY/tmux e2e 冒烟 MUST 在较大 scrollback 下仍能完成一轮并干净退出，MUST NOT 以 OS CPU% 作为硬闸。

  @req:ath26 @human
  场景: streaming-assistant-incremental-paint
    - 产品对 streaming_assistant 的 live Markdown 绘制 MUST 在 width/theme 不变且文本仅后缀增长时，复用已稳定 Markdown 前缀（段落边界，且不切开未闭合代码围栏）的已渲染行，仅对不稳定后缀（含流式 … 尾标）重新 Markdown 解析；MUST NOT 在每次 TextDelta 上都对整段 streaming 缓冲做全量解析（当已存在可复用稳定前缀时）。本要求 MUST NOT 改变 tool/bash/write/diff 的 Ctrl+O 视口折叠/展开或硬截断禁展开语义（att14/att16）。MUST 在无真 TTY harness 中可测（全量解析计数上界 + 与全量解析行一致）。

  @req:ath27 @human
  场景: mcp-discovery-surface
    - 产品 host MUST 将 MCP 发现主路径接到 `/mcp` SelectList（atm17）：refresh_loaded_resources（或等价）MUST 更新可供 `/mcp` 同步挂载的缓存快照；Driver 只读缝 MUST 能支撑列表行（每 server 连接态 + tools armed）与汇总。MCP 仍 connecting、或工具表尚未 FROZEN 且 bootstrap 未完成（含 resume 后 Settling、旧工具仍 armed）、或 Connected 尚未 armed 且未冻表时，MAY 在 busy 下轮预告位或 idle status 显示短 cue；已提交门闸 status lead=`Assembling` 时右侧 MUST 可与该 cue 并存。文案 MUST 固定为 `mcp pending (see /mcp)` 且右对齐（MUST NOT 分数计数，MUST NOT 枚举 server id）。Failed 或（bootstrap 已完成且已 FROZEN）后的未武装 MUST NOT 单独拖住短 cue（细节进 `/mcp`）。connecting 收口 / 冻表后短 cue MUST 收起（除非仍有 Connecting）。头卡 loaded-resources mcp 行仍可作启动摘要，但 MUST NOT 作为长对话唯一发现面。MUST NOT 因开 `/mcp` abort agent；MUST NOT 从 app/tui reach infra::mcp。

  @req:ath28 @human
  场景: reload-in-progress-ux
    - 产品 TUI 在 idle 启动无参 /reload 后、重载未完成前 MUST：以独立 reload 进行中态（非 agent run_active、非 bang busy）驱动 status lead 为 spinner+`Reloading`；host/effects MUST NOT 因 await 重载而阻塞终端 tick 与键入处理。进行中 MUST 允许打字与 Ctrl+G 外编；Enter 提交普通上行 / slash（含二次 /reload）/ bang MUST 拒绝且经通知条 body 恰好为 `reloading — wait`（可见 `Error: reloading — wait`），MUST NOT 调用第二次 runtime reload 或入 steer。无 overlay 时 Esc 与 Ctrl+C MUST 请求协作取消重载（非退出）；有 overlay 时 MUST 先关槽。取消收口 MUST 恢复一致工具/MCP 快照（旧 manager 保留至新装好或取消恢复）、put-back reload 句柄、解除进行中态，并以滚动提示 `Reload cancelled:`（含已完成步进摘要）+ 通知条 body `reload cancelled` 说明；已写入的 skills/context MUST NOT 要求事务回滚。失败（含墙钟超时诊断）MUST 以既有 `Reload:` 步进报告 + 通知条 body `reload failed — see report` 说明并解除进行中态。成功路径仍尾插既有 `Reload:` 汇总；MUST NOT 清空 transcript/session 历史或 editor 草稿。agent/bang 真 busy 时 /reload 拒绝保持 ath20/atm12。墙钟对齐既有 MCP 单 server 与首 turn 门闸常量。验证 MUST 含可注入慢/失败/取消的 harness。

  @req:avs1 @human
  场景: synthetic-harness-one-round
    - 产品 TUI MUST 提供可在无真 TTY/无真 LLM 下运行的合成验收：ScriptedDriver（或等价）记录 run/steer/follow_up/abort/clear_queue，并能将预置 XyEvent 流回流 HostSession。MUST 覆盖全链场景清单：提交→流式/工具→steer/follow-up queue strip→abort 后再提交→/exit 触发 finish（按当前交互模式分发 teardown；默认 ApplicationOwned）并 stop（场景清单随 harness 命名漂移，以全链覆盖语义为准）；MUST NOT 把活树、bash、compaction UI 纳入本切片。

  @req:avs2 @human
  场景: product-pty-fake-smoke
    - 仓库 MUST 含产品二进制的 PTY 冒烟（#[ignore]，经 just test-tui-e2e-pty 可跑）：在临时 Fake 模型配置与 --trust 下启动 TUI，提交用户消息后屏上 MUST 出现 Fake 默认文案 Hello from fake provider，输入 /exit 后进程 MUST 退出。默认 just qa MUST NOT 强制该用例；MUST NOT 要求跨进程 Fake 脚本化，MUST NOT 在本用例断言 steer/工具 strip。

  @req:ath29 @human
  场景: mouse-input-opt-in-no-moved-paint
    - 产品 host 扇入 MUST 能将 crossterm Event::Mouse 映射为 HostEvent::Input(InputEvent::Mouse)（Moved 可在映射前丢弃）。Ready 态处理 Mouse 时：无 UI dirty / 未消费态变 MUST NOT request_render（含无态变 Moved）；消费态变（如 ApplicationOwned 拖选）MAY request_render。产品 MUST NOT 经 TerminalGuard / `XYLITOL_TUI_MOUSE` 在进 TTY 时默认开 mouse capture；ApplicationOwned 会话 begin 后的 capture 见 ath30。teardown MUST Disable mouse capture（尽探针所能断言 mouse mode 不残留）。MUST 提供 #[ignore] PTY 用例（经 just test-tui-e2e-pty 可跑）；默认 just qa MUST NOT 强制该用例。折叠点击语义不在本要求范围。

  @req:ath30 @human
  场景: interaction-mode-application-owned-default
    - 产品 TUI MUST 在 host **启动构造时**绑定 xylitol-tui 交互模式为 ApplicationOwned（应用自管视口 / alt-screen）。缺省/未配置 MUST 为 ApplicationOwned。一次会话 MUST 只有一个主模式；MUST NOT 在会话运行中热切模式（改模式 = 结束进程或新建 HostSession，不是 mid-loop 换栈 API）。MUST NOT 把 XYLITOL_TUI_MOUSE 环境变量当作产品模式开关；MUST NOT 提供面向用户的 Inline/ApplicationOwned 切换设置。产品 MUST 将下缘固定区（至少 status/editor/footer 所占行）登记为 dock，使包级选区排除输入面；teardown MUST 不残留 mouse capture / alt-buffer。退出 ApplicationOwned 时产品 MUST 依赖库 finish dump（或等价）使主屏 scrollback 仍可读会话内容；本要求不要求向用户暴露 dump opt-out。折叠点击语义不在本要求范围（见后续 fold 族 change）。库仍可暴露 Inline 构造入口供 lab/demo；产品默认路径 MUST NOT 使用 Inline。

  @req:ath31 @human
  场景: mode-b-copy-notice-fixed-zone
    - 产品 TUI 在 ApplicationOwned 会话下，当库发出松手复制成功 copy-notice（ptim15）时 MUST 展示短时用户可见提醒（TTL 约 1.5–3s 后自动消失）。落点 SHOULD 为下缘固定区内、status/输入带附近的单行提示（或独立 info 固定区槽）；MUST NOT 写入 transcript / ScrollNotice；MUST NOT 使用带 `Error: ` 前缀的拒闸 toast-notice 形态冒充成功确认。折叠点击不在范围。

  @req:ath32 @human
  场景: enter-follows-transcript-bottom
    - 产品 TUI 在 ApplicationOwned 的 Ready 主输入面收到 `tui.input.submit` Enter 时 MUST 立即将 transcript 视口滚到底部并恢复尾插；该行为 MUST 同时适用于非空提交与空输入。空输入 MUST NOT 因此创建 submit；非 Editor 槽中的 Enter MUST 保留给该槽自身的确认语义，不得强制滚动 transcript。

  @req:ath33 @human
  场景: fold-triangle-hit-priority
    - 产品 TUI 在 ApplicationOwned 会话下 MUST 将 live scrollback 折叠三角命中表接到库 set_transcript_hit_priority（或等价）：Left Down 命中三角列时 MUST 吞按下、清除 transcript 选区且不启拖选，并触发 att20/att21 的单块 toggle；未命中时 MUST 保持既有选区/dock/Editor 路径。transcript 拖选进行中 MUST 不重新消费折叠命中。产品 MUST NOT 经 XYLITOL_TUI_MOUSE 作为折叠开关。验证 MUST 含无真 TTY harness（合成 Mouse）。

  @req:ath34 @human
  场景: attach-tick-no-blocking-rpc
    - 产品 TUI attach 路径的 host 循环在 idle/busy tick 与 drain_pending 中 MUST NOT 同步阻塞等待 Host unary（含 get_state / current_model / MCP settle）。status spinner MUST 在 Host 慢连接或写者 unary 进行中仍能换帧；键入 MUST 仍可进入 editor。写者 unary（含 SetModel）MUST NOT 在返回前等待全部 MCP 连接完成。模型列表打开与选定 MUST 在 MCP 未结算时仍可操作（选定可走 NextTurn）。

  @req:ath35 @human
  场景: attach-mux-session-lifetime
    - 产品 TUI attach MUST 在进入 host 循环前完成握手并订阅该 session 的 mux（启动即订，不等第一次 prompt）。单次 AgentEnd MUST NOT 拆掉该订阅。WS 断开后 MUST 用 last_seq 再次 subscribe 续传；收到 session/resync_required 后 MUST 按 server-core 再订并重建可见 transcript，MUST NOT 只留一行错误后放弃事件流。退出 TUI 才关闭 mux。

  @req:ath36 @human
  场景: attach-restore-projection
    - 产品 TUI attach 恢复（CLI --session 启动与空闲 Resume 切换）MUST 以 Host 消息快照一次重建 transcript：完整历史一次可见、会话 Idle、无假 spinner、无逐条回放；恢复窗内的冷订实况磁带（AgentStart / TextDelta 等回合磁带）MUST NOT 渲染进 transcript；MUST NOT 以 loading 态掩盖回放。

  @req:ath37 @human
  场景: attach-reload-cooperative-cancel
    - 产品 TUI attach 下 `/reload` 进行中用户触发取消（interrupt/clear 键位）时，client MUST 将取消意图经 Host 转达（合作取消该次进程级 reload），MUST NOT 继续等待原 reload 完成；取消后 reload 界面 MUST 以已取消收尾，Host MUST 停止后续重装步骤。正常完成路径行为保持不变。

  @req:ath38 @human
  场景: attach-fixed-zone-downlink-driven
    - 产品 TUI attach 下 MCP/skills 头卡的连接态刷新 MUST 由 Host 的 `session/resources` 下行驱动：帧到达置脏后由 tick 读本地缓存刷新；MUST NOT 以周期性 unary 轮询同一刷新。首帧到达前的初始快照 MAY 经一次性 loaded_resources unary 获取；启动与 /reload 后的既有刷新语义（ath23）保持。

  @req:ath39 @human
  场景: unary-request-bounded
    - 产品 TUI 对 Host 的每笔 unary 请求 MUST 有请求级等待界；已知长操作（如 reload）MUST 分级放宽且各有界；半开连接下 MUST 以超时错误呈现而非无限挂起。

  @req:ath40 @human
  场景: mux-halfopen-detect
    - mux 下行客户端 MUST 周期性探测连接活性（如 keepalive ping）；连续探测周期无任何入站帧时 MUST 判定半开并走既有重订/resync 路径，MUST NOT 静默停摆。

  @req:ath41 @human
  场景: attach-reconnect-backoff-generation
    - 产品 TUI attach 的 mux 重连循环 MUST 指数退避，且单次连接存活达到阈值后才 MUST 归零退避：存活不足阈值的反复闪断 MUST 按持续故障逐次升级，MUST NOT 以固定高频重试冲击 Host。重订/重连循环 MUST 携带代际计数，旧代连接的迟到帧 MUST NOT 进入新代投影。

  @req:ath42 @human
  场景: attach-reconnect-grace-ux
    - 产品 TUI attach 下初始连接与重连 MUST 各有宽限期：宽限内 MUST NOT 因断线/重试打扰信息面（transcript 与状态条零输出）；超宽限 MUST 以通知条（toast notice）提示断线重连中，恢复成功 MUST 以通知条收尾并清除断线态；重连窗内 MUST NOT 向 transcript 推错误行。

  @req:ath43 @human
  场景: attach-coalesce-downlink
    - mux 下行事件 MUST 在短窗口内攒批合帧：同窗口多条事件 MUST 合并为一次 UI 投影批消费，MUST NOT 逐事件触发投影。

  @req:ath44 @human
  场景: attach-hello-handshake
    - mux 下行连接建立后 server MUST 首帧发送 server_hello 且载荷携带现行协议版本；客户端 MUST 对每条连接校验首帧与版本：首帧缺失或不符 MUST 判连接失败进入重连判定，版本不符 MUST 按不可重试故障终止（fatal），MUST NOT 降级、MUST NOT 重试风暴；握手版本语义 MUST 单一，MUST NOT 同时维护两套版本协商。

  @req:ath41 @executable
  场景: reconnect-backoff-escalation
    假如 以注入短常量的 mock HostClient 驱动 attach 重连
    当 连续建立存活不足归零阈值即断开的连接
    那么 重试间隔 MUST 逐次翻倍升级
    当 某次连接存活达到归零阈值后断开
    那么 下次重试间隔 MUST 回落到起点

  @req:ath41 @executable
  场景: reconnect-stale-generation-dropped
    假如 已有一条订阅中的 mock mux 连接
    当 触发重订或换会话产生新代循环后旧代连接迟到推入事件
    那么 该迟到事件 MUST NOT 出现在新代的 drain 结果中

  @req:ath43 @executable
  场景: attach-coalesce-burst-single-projection
    假如 合帧窗口开启
    当 窗口内连续到达多条下行事件
    那么 客户端 MUST 以一次投影批消费且消费批数等于合并后批数而非事件条数

  @req:ath44 @executable
  场景: attach-hello-mismatch-fatal
    假如 mock HostClient 在 mux 首帧发送版本不符的 server_hello
    当 attach 客户端完成首帧校验
    那么 driver MUST 报版本错误并终止该代循环且 MUST NOT 进入重试循环

  @req:ath44 @executable
  场景: real-kill-reconnect-journal-resume
    假如 真进程 serve 已启动且 attach 客户端已订阅
    当 server 的 mux 连接被断开且期间 journal 新增事件
    那么 客户端 MUST 在宽限内不上屏断线错误并自动重连
    并且 重连后 MUST 按 last_seq 从 journal 续传缺失事件且 MUST NOT 依赖人工重开
