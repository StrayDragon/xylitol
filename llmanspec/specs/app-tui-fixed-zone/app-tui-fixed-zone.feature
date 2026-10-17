# language: zh-CN
# purpose: 主题 token、glyph、status、footer、通知条、通知栈、待办栏等固定区（非滚动 layout 壳）行为。

功能: app-tui-fixed-zone

  @req:r1214
  规则: idle-status-omitted
    产品 UiRoot 空闲时 status 槽 须渲染 0 行；忙碌时 须至多独立 1 行短状态；不得将 Working 等忙碌文案并入 footer。

    场景: idle-status-empty-busy-single-line
      当 以空闲与忙碌两种 UiModel 渲染固定区
      那么 空闲帧不含忙碌短词且忙碌帧恰一行 Working 且 footer 不含 Working
  @req:r1224
  规则: footer-minimal
    产品 footer 须为 1 行 dim 文本，显示 active（in-flight 或当前 turn 已绑定）的 model，而非仅裸读 selected；字段序为 cwd · model · thinking 标签（形如 `cwd · model · thinking off` 或 `· xhigh`，level 文案为该档位的显示名，off 时用 `thinking off`；字段间只用 ` · `，不得在 thinking 前再加装饰 `•`）；当 active 模型无思考或仅不可调 off 时 须省略 thinking 段；并在可获得 ContextTokenEstimate 时追加 `· used … tokens`（文案按 TokenProvenance：Api 为 used C tokens，Heuristic 为 used ~C tokens，Unknown 为 used ? tokens；其中 C 为与 window 同源的紧凑计数，见 atc24）；当当前模型 context_window > 0 时 须在 used 字段后追加派生占用比（见 atc21）；不得在 footer 展示队列计数徽章（如 `q:sN|fM`；队列可见性见 ati11 / atc26）；无估计数据时 不得伪造 used 0；不得常驻快捷键墙；放不下时 须截断右侧而非增高。

    场景: footer-single-line-with-heuristic-provenance
      当 以 Heuristic 上下文估计驱动 footer token 字段
      那么 footer 为单行且含 used ~C tokens 波浪号且无队列徽章
  @req:r1233
  规则: theme-glyphs-config
    产品 host 须经 Palette::dark()（或等价闭包）将语义色注入组件主题；glyph 档（unicode/ascii）须由显式配置切换，不得做运行时字体探测；产品 MVP 不得默认开启 theme auto。
    场景: glyph-set-by-config-not-font-probe
      假如 折叠字形环境未指定（默认 Unicode 集）
      当 读取折叠与展开字形
      那么 折叠为 ▸ 展开为 ▾ 且各占单列
      并且 切换环境变量 XYLITOL_TUI_GLYPH_SET=ascii 并重新读取
      并且 折叠回退为 > 展开回退为 v 且各占单列

    场景: semantic-colors-from-single-token-source
      当 读取 token 检查脚本与色板
      那么 单一脚本对照 DESIGN 与 Palette
  @req:r1234
  规则: design-docs-ssot
    产品 TUI 视觉须以 DESIGN.md frontmatter 为书面色板 SSOT，Palette 为运行时快照；组件行为以产品代码与面 AGENTS 为准。不得另立平行设计稿目录作为运行时真值。

    场景: token-check-is-a-gate
      当 读取 token 闸接线
      那么 token 闸由 qa-check-tui-tokens 执行

    场景: surface-agents-points-to-written-docs
      当 读取 tui 面 AGENTS 摘要
      那么 摘要写明先读产品代码与 DESIGN.md
      并且 摘要写明改色板须跑 token 闸
  @req:r1235
  规则: demo-theme-auto-detect
    agent_demo 须默认使用 Dark token 集；当启用主题自动探测（环境变量或 harness API）时 须经 package terminal_colors 纯函数切换 Light/Dark，并在 layout 壳（footer 或系统行）暴露当前 theme_mode；不得将自动切换设为产品 MVP 默认。

    场景: demo-defaults-dark-with-explicit-auto
      当 读取 demo 主题探测接线
      那么 缺省为 dark 且自动切换需显式开启
  @req:r1236
  规则: idle-editor-compact
    产品空闲且 Editor 草稿为空时操作区可见内容行 须紧凑（对齐 agent_demo 观感：上下 ─ 保留，避免大块空盒）；有多行草稿时 须仍可按既有 terminal_rows/max_vis 规则长高。

    场景: idle-editor-zone-keeps-compact-border
      当 以空闲 UiModel 渲染固定区
      那么 编辑器操作区保留上下横线边框且无大块空盒
  @req:r1237
  规则: busy-status-spinner
    产品 agent-busy status 行 须为至多一行：左侧 lead 为 accent spinner（或等价动画帧）加短词（须作为同一组合贴左，不得用 space-between 拆开 spinner 与短词）；idle 须仍为 0 行；不得把 spinner 画进 footer。

    场景: busy-spinner-single-row-with-lead
      当 以忙碌 UiModel 渲染固定区
      那么 恰一行含 spinner 短词且 footer 不含该短词
  @req:r1238
  规则: user-message-no-wash
    产品 scrollback 用户消息行默认 不得应用 DESIGN user-message-bg 全行淡底；须保留消息文本语义前缀（如 ❯ / glyph）；不得为用户行画 status 轨。

    场景: user-message-keeps-prefix-no-wash
      当 以含用户消息的 UiModel 渲染 scrollback
      那么 用户行保留语义前缀与正文
  @req:r1239
  规则: compaction-retry-single-status-line
    产品 TUI 在 Compacting 或 Retry n/m 忙碌态时 status 槽 须仍至多独立 1 行（spinner+短词）；压缩/重试细节 须走 scrollback 滚动提示（ScrollNotice）而非增高 status；不得常驻多行 compaction/retry layout 条。

    场景: compacting-status-stays-single-row
      当 以 Compacting 忙碌态 UiModel 渲染固定区
      那么 恰一行 Compacting 且 footer 不含 Compacting
  @req:r1215
  规则: abort-status-idle
    产品在用户 abort 后 status 槽 须回到 0 行（idle）；取消说明 须走 scrollback 滚动提示一行而非增高 status；bang 执行中 MAY 使用单行 Running。

    场景: abort-returns-status-to-idle
      当 以主机泵开启忙碌流并按下 Esc 后收流关闭并渲染
      那么 帧内不再含忙碌短词且滚动提示含取消说明一行
  @req:r1216
  规则: models-picker-footer
    经 models 槽或 /model <id> 成功更新 selected 后：idle 时 footer model/thinking 须反映新 active（与 selected 收敛）；agent-busy 时 footer 须立即反映当前 selected（run 绑定语义：本轮生成仍用开跑时绑定，由 Host 保证）；产品面 不得渲染换模预告（Next turn cue 已随 attach run 绑定退役）。无思考模型 须省略 footer thinking 段。

    场景: models-picker-footer-reflects-active
      当 以主机泵在 idle 提交 "/model fake"
      并且 渲染当前主机帧
      那么 footer 反映新 active 模型且无换模预告
  @req:r1217
  规则: status-spacer-above-editor
    产品 TUI status 槽：busy 时 须渲染 Loader 前导空行加一行 spinner 短词并紧贴 editor 上方（不得 strip 空行）；idle 时 须保留恰好一行空白呼吸间距（不得显示 Ready 文案）；对齐 agent_demo status_lines 与 status.md。

    场景: idle-no-ready-wording
      当 以空闲 UiModel 渲染固定区
      那么 帧内不出现 Ready 文案
  @req:r1219
  规则: footer-token-refresh
    产品 host 须在 session tree travel 换叶、CompactionEnd、turn 收尾 settlement（经 TurnEnd 携带或独立 ContextTokenSettlement 事件或 Driver 缓存）、以及 turn 进行中有可用 Api usage 更新时（节流）刷新 footer token 字段；刷新 须经 settlement snapshot 或 Driver 只读 estimate_context_tokens（或等价 seam）且异步不阻塞输入；同一轮 agent run 在已应用 TurnSettled settlement 后，stream close / AgentEnd 不得再触发第二次 estimate；不得在每个 TextDelta 上全量重算上下文估计。

    场景: footer-token-refresh-single-estimate
      当 以固定估计驱动一次 footer token 刷新
      那么 驱动估计恰被调用一次
  @req:r1220
  规则: 主题热应用
    产品 TUI 须能按主题名（至少内建 dark/light）将 Palette 应用到 LayoutTheme/UiRoot；未知或非法名 须保留旧主题并报告诊断；重载 不得清空 transcript。

    场景: unknown-theme-keeps-old-and-diagnoses
      当 以主机泵在 idle 提交 "/theme no-such-theme"
      那么 报告诊断且界面仍正常渲染
  @req:r1221
  规则: theme-slash-slot
    产品 /theme 无参 须使用独立 editor 槽（Themes SelectList，对齐 Models 槽模式）挂载内建主题项；选中 须调用已有主题热应用路径（atc15）并关闭槽；Esc 须关闭槽且 不得改主题；不得解冻 Plate/Settings/Choice 活板；不得将 theme auto 或 demo Ctrl+P theme-toggle 设为产品默认。

    场景: theme-slot-esc-closes-without-change
      当 以主机泵在 idle 提交 "/theme"
      那么 主题列表槽打开
      当 关闭当前槽
      那么 槽回到编辑器且未报错
  @req:r1222
  规则: thinking-border-and-footer
    产品 TUI 须将当前 Driver thinking level 映射为 ThinkingBorderLevel 并经 apply_thinking_border（或等价）染色 Editor 操作区边框；footer 须同步显示对应 level 标签（见 atc2）；主题切换后 须按新 Palette 重涂 thinking 边框；bash 强调色边框（ati15）开启时 须覆盖 thinking 边框，退出 bash 前缀后 须恢复当前 thinking 边框而非仅 muted。

    场景: thinking-level-shown-in-footer
      当 以布局根设置 thinking 档 "high" 后渲染
      那么 footer 含该档位标签
  @req:r1223
  规则: loaded-resources-slot
    产品 UiRoot 须在 scrollback 上方渲染 loaded-resources 槽：Codex 风边框卡片（>_ xylitol + model/directory；有 skills 时 skill-ref 色 skills 行并换行全量；有 MCP 时 success 色 mcp 行）；MCP 连接进行中时 mcp 行 须显示可区分的 connecting 进度（含已配置数或 i/n 与当前 server id 或等价），完成后 须收敛为 connected 摘要（已连接 id 与工具数或等价）。Host 侧已有成功连接时 不得把头卡停在仅 `N configured · 0 connected` 且无 connecting 进度、无失败诊断。失败诊断 须可感且 不得用逐步滚动提示刷墙。不得展示木糖醇中文标签；不得渲染 ASCII logo 艺术字；不得用省略号截断资源名；不得列出或暗示存在 prompt templates（产品无此能力）；不得展示密钥或完整 env。snapshot 携带 obs_diag（观测通道短诊断）时卡 须追加一行 obs 诊断（与 mcp 失败诊断同形：单短行、无密钥、无完整 env）；absent 时 不得占行。

    场景: loaded-resources-card-renders-skills-and-mcp
      当 以含 skills 与 connecting MCP 的资源快照渲染头部卡
      那么 卡含 skills 行与 mcp 行且展示连接进度
  @req:r1226
  规则: footer-derived-context-percent
    当 footer 已展示 ContextTokenEstimate 且当前模型 context_window > 0 时，须在 used 字段后追加派生占用比：percent = tokens/context_window*100（展示 1 位小数）与紧凑 window（如 128k），形如 ` · 32.8%/128k`（used 侧紧凑例：`used 42k tokens · 32.8%/128k`）；Heuristic 须为 ` · ~p%/W`；Unknown 且有窗 须为 ` · ?%/W`；context_window 为 0 或不展示 used 字段时 不得追加 %；该百分比 须仅作展示，不得作为 should_compact / reserve 触发 SSOT，不得恢复 compaction_threshold 或百分比闸配置。

    场景: footer-derived-percent-from-api-usage
      当 以 Api 来源 42k tokens 与 128k 窗口驱动 footer
      那么 footer 含 used 42k tokens 与 32.8%/128k 派生占用比
  @req:r1227
  规则: toast-notice-ephemeral
    产品 TUI 须提供通知条（toast notice）：固定于固定区、位于 status/spinner 槽上方恰好一行；前景 须用 warning token（非 muted、非 accent）；可见文案 须以字面前缀 `Error: ` 开头后接 body；不得写入 UiModel.entries / UiEntry；新通知 须替换旧通知（单槽）；显示后 须在约定 TTL（默认约 3–5s）内经 idle_tick/step 自动清除。agent/bang busy 下 Resume 面板 switch/rename/delete 拒闸（见 atm10）须走通知条，不得再追加 UiEntry::ScrollNotice。不得用通知条冒充对话正文或下轮预告。

    场景: toast-single-slot-replaces-with-error-prefix
      当 连续推送两条通知条后渲染
      那么 仅见后一条且以 Error: 前缀且未写入对话条目
  @req:r1930
  规则: client-notice-stack
    产品 TUI 须在视口右上角提供客户端通知栈：纵向堆叠、最新在上、同时最多约 3 条，TTL 到期自动消失；成功确认（至少复制成功 Copied、Copied Last Msg）以 ✓ 起头，说明类（至少整段 Heuristic 估计的首次提示）以 ◆ 起头。该右上角 须为当前已裁切的可见 transcript 视口，滚动对话后 须仍钉在该角；不得写在滚动内容缓冲第 0 行并随内容滚出视口。不得写入对话条目 / ScrollNotice；不得计入下缘 Fixed-Zone Footprint；不得使用通知条的 Error: 前缀。不得把导航瞬时（switched →、forked →、cloned →、imported →、restored →、new session →、history @）放入本栈（那些走滚动提示尾插，见 r1953 / r1330）。整段估计 provenance 为 Heuristic 时 须在每个 TUI 进程内至多提示一次；Api 锚点有效仅 trailing 为启发式时 不得再弹。Print 面 不得展示该栈。本波 不得要求栈内键盘动作或展开侧板。
    场景: notice-stack-copied-not-error-bar
      当 臂装复制成功提示后渲染主机帧
      那么 右上角通知栈含 Copied 且不以 Error 前缀冒充
    场景: notice-stack-heuristic-once
      假如 footer 同源估计为 Heuristic
      当 连续两次以 Heuristic 刷新估计
      那么 估算说明在通知栈至多出现一次
    场景: notice-stack-copied-pins-scrolled-viewport
      假如 对话内容高于可见 transcript 窗且视口未停在内容顶
      当 臂装复制成功提示后渲染已裁切视口
      那么 可见视口顶部含 Copied 且不以 Error 前缀冒充
  @req:r1228
  规则: fixed-zone-footprint-term-budget
    产品 TUI 须维护 Fixed-Zone Footprint（单表或单函数 SSOT）：按终端行高 term_rows 为下缘固定区预留最小行（busy status 按前导空行+短词计 2 行、footer 1 行；非空 queue strip / 通知条 / 待办栏各按其实际行计入；右上角通知栈 不得计入该下缘预留）。EditorSlot 内带 max_visible 的列表/树（至少 Resume、Tree、Models、MCP、Themes、Import）body 可见行顶 须 = term_rows 减去上述 reserved 与槽头行后的预算且 须 ≥ 1；不得以与 term_rows 脱节的硬编码 10 作为运行时顶。content-end 视口下，短终端 agent-busy 且上述高槽打开时，视口内 须仍能看到 status lead（Working 或等价 spinner 短词）。不得为本需求改 xylitol-tui content-end 视口语义或引入引擎级 bottom-fixed-zone pin（除非另开变更）。

    场景: short-terminal-busy-lead-visible
      当 以小高度终端渲染忙碌帧
      那么 status lead 短词在视口内可见
  @req:r1229
  规则: footer-used-compact-count
    footer used 字段中的可数 token 计数 C（atc2/atc13）须与 context_window 紧凑展示使用同一算法（实现单点，勿另立第二套阈值）：C<1000 须为十进制全量；1000≤C<10000 须为一位小数的 k（如 1.0k）；10000≤C<1000000 须为四舍五入整 k（如 42k）；更大 须用 M 同理。Unknown 的 `?` 不得套用紧凑。不得去掉 tokens 词。

    场景: footer-used-compact-k-notation
      当 以 42000 与 8123456 两种 token 数驱动 footer
      那么 分别展示 used 42k tokens 与 used 8.1M tokens 且保留 tokens 词
  @req:r1230
  规则: reload-status-and-toasts
    产品 TUI 在 /reload 进行中（ath28 reload 态）时 status 槽 须显示 Loader 形态：前导空行 + 一行 spinner+短词 `Reloading`（lead 贴左，与 atc7/atc12 同形）；不得冒充 agent Working 或 Assembling；该态下 status 右侧 不得显示 mcp pending cue 或任何换模预告。软闸拒提交、取消收口、失败收口的通知条 须走 atc22：body 分别为 `reloading — wait`、`reload cancelled`、`reload failed — see report`（可见前缀 `Error: `）；取消与失败 须另有滚动提示报告（见 ath28），软闸拒提交 不得为此追加 ScrollNotice。reload 结束后 status 须回到 idle 0 内容行（保留呼吸空行）。

    场景: reload-status-lead-reloading
      当 以主机泵进入 reload 进行中态后渲染
      那么 status 显示 Reloading 短词且不冒充 Working
  @req:r1231
  规则: fixed-zone-no-extra-undocumented
    产品 TUI 固定区（footer / status / 队列条 / 通知条 / 待办栏等）可观察文案与徽章 须仅来自文档化视觉 SSOT（见 atc4）已声明的槽与字段；不得另加 SSOT 未声明或已废弃的冗余展示（避免用户疑惑）；队列可见性 SSOT 为中间队列条（Steering:/Follow-up:，见 ati11），不得再在 footer/status 重复队列计数徽章。

    场景: fixed-zone-slot-order
      当 产品 TUI 同时存在非空待办栏、队列条与通知条
      那么 从上到下 为队列条、待办栏、通知条、status、editor

    场景: footer-carries-no-queue-badge
      当 以 Heuristic 上下文估计驱动 footer token 字段
      那么 footer 为单行且含 used ~C tokens 波浪号且无队列徽章
  @req:r1232
  规则: tool-header-timeout-note
    命令类工具（bash/grep/find）的模型显式 timeout 请求 须在工具行 header 的按键提示 (Alt+E) 前以 muted 文本显示 (timeout {N}s)（N 为钳制后的生效秒数）；省略（走工具默认）时 不得显示。注记为静态文本，不得做倒计时或运行中改写。

    场景: tool-header-timeout-note-shown-when-requested
      当 以带与不带 timeout 的工具条目渲染 scrollback
      那么 带 30s 的显示 (timeout 30s) 且省略的不显示 timeout 注记
  @req:r21
  规则: todo-bar-dock-member
    产品 TUI 待办栏 须为下缘固定区成员：有 Todo 条目时驻留于队列条与通知条之间，通知条仍紧贴 status、status 仍紧贴 editor；空表 须占 0 行。待办栏 不得写入对话条目，不得冒充 status、footer、通知条或队列条。折叠/展开/换行续行与上下轮廓（含 `↑ N more` / `↓ N more` 边框）须计入 Fixed-Zone Footprint 实际行（可见行，含高度上限后的内容视口高加轮廓，不含未滚入内容）。

    场景: todo-bar-sits-between-queue-and-toast
      当 产品 TUI 同时存在非空待办栏、队列条与通知条
      那么 从上到下 为队列条、待办栏、通知条、status、editor
      并且 空表时待办栏 占 0 行

  @req:r2903
  规则: 会话详情覆盖弹出框
    产品 TUI 须提供屏幕右侧会话详情覆盖弹出框（overlay，非常驻分栏，DESIGN 硬规则 2 例外）：键位切换显隐，展示会话身份、生效模型、thinking 档与工作目录；复按切换或 Esc 须收起；通知栈 z 序须高于覆盖弹出框。覆盖弹出框为瞬态展示，不得冒充待办栏、status 或槽，不得常驻占用版面。

    场景: session-popup-toggle-render-close
      假如 产品 TUI 主循环就绪
      当 切换覆盖弹出框显隐
      那么 覆盖弹出框渲染会话详情
      当 复按切换收起
      那么 覆盖弹出框不再渲染

    场景: popup-idle-esc-close-no-tree-chain
      假如 产品 TUI 主循环就绪
      当 切换覆盖弹出框显隐
      并且 按 Esc 收起覆盖弹出框
      并且 再按 Esc
      那么 覆盖弹出框不再渲染且会话树未打开

    场景: popup-busy-esc-passes-abort
      假如 产品 TUI 主循环就绪
      当 切换覆盖弹出框显隐
      并且 忙碌中按 Esc
      那么 中止计一次且覆盖弹出框仍渲染

  @req:r2904
  规则: 会话侧栏
    产品 TUI 须提供常驻会话侧栏：独立键位切换；开启时内容区 须按全宽减侧栏宽重排（换行按内容宽，不得以覆盖截断冒充让宽），最右整高绘制分隔与实验占位内容（现施工中；业务字段待正式业务代码定义后回归，见 c2911 草案）；viewport 通知栈与瞬态覆盖弹出框 须按内容宽收口，不得压住侧栏；关闭后内容区 须恢复全宽。侧栏开启时滚动槽（gutter）须贴内容右边界（侧栏左缘），侧栏带内 不得渲染滚动条；鼠标滚轮与选区 须以内容边界为界，不得越界作用于侧栏带。侧栏与覆盖弹出框显隐互不影响；不得冒充待办栏或槽。侧栏 仅标准档及以上（宽度档位 ≥80 列）可用，标准档以下 不得渲染侧栏；标准档以下按切换键 须降级为打开覆盖弹出框（信息不丢、形态切换）。宿主让宽对账 须与侧栏布局同口径推导——开启且可用才让宽。

    场景: session-sidebar-toggle-and-restore
      假如 产品 TUI 主循环就绪
      当 切换侧栏
      那么 右侧栏上屏且内容区让宽
      当 复按收起
      那么 侧栏消失且内容区恢复

    场景: sidebar-below-standard-tier-falls-back-to-popup
      假如 产品 TUI 会话就绪于窄终端
      当 切换侧栏
      那么 覆盖弹出框上屏且侧栏不出

    场景: sidebar-and-popup-coexist
      假如 产品 TUI 主循环就绪
      当 先开覆盖弹出框再开侧栏
      那么 两面同时可辨且覆盖弹出框贴内容右边界
