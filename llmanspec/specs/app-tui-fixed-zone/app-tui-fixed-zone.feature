# language: zh-CN
# capability: app-tui-fixed-zone
# purpose: 主题 token、glyph、status、footer、通知条、待办栏等固定区（非滚动 layout 壳）行为。
# scope: src/app/tui/

功能: app-tui-fixed-zone

  @req:r1214 @human
  场景: idle-status-omitted
    - 产品 UiRoot 空闲时 status 槽 MUST 渲染 0 行；忙碌时 MUST 至多独立 1 行短状态；MUST NOT 将 Working 等忙碌文案并入 footer。

  @req:r1224 @human
  场景: footer-minimal
    - 产品 footer MUST 为 1 行 dim 文本，显示 active（in-flight 或当前 turn 已绑定）的 model，而非仅裸读 selected；字段序为 cwd · model · thinking 标签（形如 `cwd · model · thinking off` 或 `· xhigh`，level 文案为该档位的显示名，off 时用 `thinking off`；字段间只用 ` · `，MUST NOT 在 thinking 前再加装饰 `•`）；当 active 模型无思考或仅不可调 off 时 MUST 省略 thinking 段；并在可获得 ContextTokenEstimate 时追加 `· used … tokens`（文案按 TokenProvenance：Api/RemoteCount/LocalTokenizer 为 used C tokens，Heuristic 为 used ~C tokens，Unknown 为 used ? tokens；其中 C 为与 window 同源的紧凑计数，见 atc24）；当当前模型 context_window > 0 时 MUST 在 used 字段后追加派生占用比（见 atc21）；MUST NOT 在 footer 展示队列计数徽章（如 `q:sN|fM`；队列可见性见 ati11 / atc26）；无估计数据时 MUST NOT 伪造 used 0；MUST NOT 常驻快捷键墙；放不下时 MUST 截断右侧而非增高。

  @req:r1233 @human
  场景: theme-glyphs-config
    - 产品 host MUST 经 Palette::dark()（或等价闭包）将语义色注入组件主题；glyph 档（unicode/ascii）MUST 由显式配置切换，MUST NOT 做运行时字体探测；产品 MVP MUST NOT 默认开启 theme auto。

  @req:r1234 @human
  场景: design-docs-ssot
    - 产品 TUI 视觉与 UX MUST 有单一书面 SSOT（总索引 + 组件级子文档）；实现与 demo 引用组件规则时 MUST 指向对应书面文档（尤其 diff-block 与 keybindings），MUST NOT 仅依赖口头约定。文档落点（总索引与子文档目录）由面 AGENTS 维护。

  @req:r1235 @human
  场景: demo-theme-auto-detect
    - agent_demo MUST 默认使用 Dark token 集；当启用主题自动探测（环境变量或 harness API）时 MUST 经 package terminal_colors 纯函数切换 Light/Dark，并在 layout 壳（footer 或系统行）暴露当前 theme_mode；MUST NOT 将自动切换设为产品 MVP 默认。

  @req:r1236 @human
  场景: idle-editor-compact
    - 产品空闲且 Editor 草稿为空时操作区可见内容行 MUST 紧凑（对齐 agent_demo 观感：上下 ─ 保留，避免大块空盒）；有多行草稿时 MUST 仍可按既有 terminal_rows/max_vis 规则长高。

  @req:r1237 @human
  场景: busy-status-spinner
    - 产品 agent-busy status 行 MUST 为至多一行：左侧 lead 为 accent spinner（或等价动画帧）加短词（MUST 作为同一组合贴左，MUST NOT 用 space-between 拆开 spinner 与短词）；idle MUST 仍为 0 行；MUST NOT 把 spinner 画进 footer。

  @req:r1238 @human
  场景: user-message-no-wash
    - 产品 scrollback 用户消息行默认 MUST NOT 应用 DESIGN user-message-bg 全行淡底；MUST 保留消息文本语义前缀（如 ❯ / glyph）；MUST NOT 为用户行画 status 轨。

  @req:r1239 @human
  场景: compaction-retry-single-status-line
    - 产品 TUI 在 Compacting 或 Retry n/m 忙碌态时 status 槽 MUST 仍至多独立 1 行（spinner+短词）；压缩/重试细节 MUST 走 scrollback 滚动提示（ScrollNotice）而非增高 status；MUST NOT 常驻多行 compaction/retry layout 条。

  @req:r1215 @human
  场景: abort-status-idle
    - 产品在用户 abort 后 status 槽 MUST 回到 0 行（idle）；取消说明 MUST 走 scrollback 滚动提示一行而非增高 status；bang 执行中 MAY 使用单行 Running。

  @req:r1216 @human
  场景: models-picker-footer
    - 经 models 槽或 /model <id> 成功更新 selected 后：idle 时 footer model/thinking MUST 反映新 active（与 selected 收敛）；agent-busy 时 footer MUST 立即反映当前 selected（run 绑定语义：本轮生成仍用开跑时绑定，由 Host 保证）；产品面 MUST NOT 渲染换模预告（Next turn cue 已随 attach run 绑定退役）。无思考模型 MUST 省略 footer thinking 段。

  @req:r1217 @human
  场景: status-spacer-above-editor
    - 产品 TUI status 槽：busy 时 MUST 渲染 Loader 前导空行加一行 spinner 短词并紧贴 editor 上方（MUST NOT strip 空行）；idle 时 MUST 保留恰好一行空白呼吸间距（MUST NOT 显示 Ready 文案）；对齐 agent_demo status_lines 与 status.md。

  @req:r1218 @human
  场景: footer-token-provenance
    - 当 footer 展示上下文 token 占用时，文案 MUST 反映 Driver::estimate_context_tokens 返回的 TokenProvenance：精确来源（Api/RemoteCount/LocalTokenizer）MUST 使用无波浪号的 used C tokens；Heuristic MUST 使用 used ~C tokens；Unknown MUST 使用 used ? tokens；MUST NOT 将 Heuristic 显示为无 ~ 的精确值；C 的紧凑规则见 atc24；派生占用比的波浪号规则见 atc21。

  @req:r1219 @human
  场景: footer-token-refresh
    - 产品 host MUST 在 session tree travel 换叶、CompactionEnd、turn 收尾 settlement（经 TurnEnd 携带或独立 ContextTokenSettlement 事件或 Driver 缓存）、以及 turn 进行中有可用 Api usage 更新时（节流）刷新 footer token 字段；刷新 MUST 经 settlement snapshot 或 Driver 只读 estimate_context_tokens（或等价 seam）且异步不阻塞输入；同一轮 agent run 在已应用 TurnSettled settlement 后，stream close / AgentEnd MUST NOT 再触发第二次 estimate；MUST NOT 在每个 TextDelta 上全量本地 tokenizer.encode。

  @req:r1220 @human
  场景: 主题热应用
    - 产品 TUI MUST 能按主题名（至少内建 dark/light）将 Palette 应用到 LayoutTheme/UiRoot；未知或非法名 MUST 保留旧主题并报告诊断；重载 MUST NOT 清空 transcript。

  @req:r1221 @human
  场景: theme-slash-slot
    - 产品 /theme 无参 MUST 使用独立 editor 槽（Themes SelectList，对齐 Models 槽模式）挂载内建主题项；选中 MUST 调用已有主题热应用路径（atc15）并关闭槽；Esc MUST 关闭槽且 MUST NOT 改主题；MUST NOT 解冻 Plate/Settings/Choice 活板；MUST NOT 将 theme auto 或 demo Ctrl+P theme-toggle 设为产品默认。

  @req:r1222 @human
  场景: thinking-border-and-footer
    - 产品 TUI MUST 将当前 Driver thinking level 映射为 ThinkingBorderLevel 并经 apply_thinking_border（或等价）染色 Editor 操作区边框；footer MUST 同步显示对应 level 标签（见 atc2）；主题切换后 MUST 按新 Palette 重涂 thinking 边框；bash 强调色边框（ati15）开启时 MUST 覆盖 thinking 边框，退出 bash 前缀后 MUST 恢复当前 thinking 边框而非仅 muted。

  @req:r1223 @human
  场景: loaded-resources-slot
    - 产品 UiRoot MUST 在 scrollback 上方渲染 loaded-resources 槽：Codex 风边框卡片（>_ xylitol + model/directory；有 skills 时 skill-ref 色 skills 行并换行全量；有 MCP 时 success 色 mcp 行）；MCP 连接进行中时 mcp 行 MUST 显示可区分的 connecting 进度（含已配置数或 i/n 与当前 server id 或等价），完成后 MUST 收敛为 connected 摘要（已连接 id 与工具数或等价）。Host 侧已有成功连接时 MUST NOT 把头卡停在仅 `N configured · 0 connected` 且无 connecting 进度、无失败诊断。失败诊断 MUST 可感且 MUST NOT 用逐步滚动提示刷墙。MUST NOT 展示木糖醇中文标签；MUST NOT 渲染 ASCII logo 艺术字；MUST NOT 用省略号截断资源名；MUST NOT 列出或暗示存在 prompt templates（产品无此能力）；MUST NOT 展示密钥或完整 env。snapshot 携带 obs_diag（观测通道短诊断）时卡 MUST 追加一行 obs 诊断（与 mcp 失败诊断同形：单短行、无密钥、无完整 env）；absent 时 MUST NOT 占行。

  @req:r1225 @human
  场景: fixed-zone-success-no-system
    - 产品 host 成功切换 model、thinking 或 theme 时 MUST NOT 向 scrollback 追加成功确认类滚动提示（如 model → …）；失败与诊断 MAY 写滚动提示（ScrollNotice）。

  @req:r1226 @human
  场景: footer-derived-context-percent
    - 当 footer 已展示 ContextTokenEstimate 且当前模型 context_window > 0 时，MUST 在 used 字段后追加派生占用比：percent = tokens/context_window*100（展示 1 位小数）与紧凑 window（如 128k），形如 ` · 32.8%/128k`（used 侧紧凑例：`used 42k tokens · 32.8%/128k`）；Heuristic MUST 为 ` · ~p%/W`；Unknown 且有窗 MUST 为 ` · ?%/W`；context_window 为 0 或不展示 used 字段时 MUST NOT 追加 %；该百分比 MUST 仅作展示，MUST NOT 作为 should_compact / reserve 触发 SSOT，MUST NOT 恢复 compaction_threshold 或百分比闸配置。

  @req:r1227 @human
  场景: toast-notice-ephemeral
    - 产品 TUI MUST 提供通知条（toast notice）：固定于固定区、位于 status/spinner 槽上方恰好一行；前景 MUST 用 warning token（非 muted、非 accent）；可见文案 MUST 以字面前缀 `Error: ` 开头后接 body；MUST NOT 写入 UiModel.entries / UiEntry；新通知 MUST 替换旧通知（单槽）；显示后 MUST 在约定 TTL（默认约 3–5s）内经 idle_tick/step 自动清除。agent/bang busy 下 Resume 面板 switch/rename/delete 拒闸（见 atm10）MUST 走通知条，MUST NOT 再追加 UiEntry::ScrollNotice。MUST NOT 用通知条冒充对话正文或下轮预告。

  @req:r1228 @human
  场景: fixed-zone-footprint-term-budget
    - 产品 TUI MUST 维护 Fixed-Zone Footprint（单表或单函数 SSOT）：按终端行高 term_rows 为下缘固定区预留最小行（busy status 按前导空行+短词计 2 行、footer 1 行；非空 queue strip / 通知条 / 待办栏各按其实际行计入）。EditorSlot 内带 max_visible 的列表/树（至少 Resume、Tree、Models、MCP、Themes、Import）body 可见行顶 MUST = term_rows 减去上述 reserved 与槽头行后的预算且 MUST ≥ 1；MUST NOT 以与 term_rows 脱节的硬编码 10 作为运行时顶。content-end 视口下，短终端 agent-busy 且上述高槽打开时，视口内 MUST 仍能看到 status lead（Working 或等价 spinner 短词）。MUST NOT 为本需求改 xylitol-tui content-end 视口语义或引入引擎级 bottom-fixed-zone pin（除非另开变更）。

  @req:r1229 @human
  场景: footer-used-compact-count
    - footer used 字段中的可数 token 计数 C（atc2/atc13）MUST 与 context_window 紧凑展示使用同一算法（实现单点，勿另立第二套阈值）：C<1000 MUST 为十进制全量；1000≤C<10000 MUST 为一位小数的 k（如 1.0k）；10000≤C<1000000 MUST 为四舍五入整 k（如 42k）；更大 MUST 用 M 同理。Unknown 的 `?` MUST NOT 套用紧凑。MUST NOT 去掉 tokens 词。

  @req:r1230 @human
  场景: reload-status-and-toasts
    - 产品 TUI 在 /reload 进行中（ath28 reload 态）时 status 槽 MUST 显示 Loader 形态：前导空行 + 一行 spinner+短词 `Reloading`（lead 贴左，与 atc7/atc12 同形）；MUST NOT 冒充 agent Working 或 Assembling；该态下 status 右侧 MUST NOT 显示 mcp pending cue 或任何换模预告。软闸拒提交、取消收口、失败收口的通知条 MUST 走 atc22：body 分别为 `reloading — wait`、`reload cancelled`、`reload failed — see report`（可见前缀 `Error: `）；取消与失败 MUST 另有滚动提示报告（见 ath28），软闸拒提交 MUST NOT 为此追加 ScrollNotice。reload 结束后 status MUST 回到 idle 0 内容行（保留呼吸空行）。

  @req:r1231 @human
  场景: fixed-zone-no-extra-undocumented
    - 产品 TUI 固定区（footer / status / 队列条 / 通知条 / 待办栏等）可观察文案与徽章 MUST 仅来自文档化视觉 SSOT（见 atc4）已声明的槽与字段；MUST NOT 另加 SSOT 未声明或已废弃的冗余展示（避免用户疑惑）；队列可见性 SSOT 为中间队列条（Steering:/Follow-up:，见 ati11），MUST NOT 再在 footer/status 重复队列计数徽章。

  @req:r1232 @human
  场景: tool-header-timeout-note
    - 命令类工具（bash/grep/find）的模型显式 timeout 请求 MUST 在工具行 header 的按键提示 (Alt+E) 前以 muted 文本显示 (timeout {N}s)（N 为钳制后的生效秒数）；省略（走工具默认）时 MUST NOT 显示。注记为静态文本，MUST NOT 做倒计时或运行中改写。

  @req:r21 @human
  场景: todo-bar-dock-member
    - 产品 TUI 待办栏 MUST 为下缘固定区成员：有 Todo 条目时驻留于队列条与通知条之间，通知条仍紧贴 status、status 仍紧贴 editor；空表 MUST 占 0 行。待办栏 MUST NOT 写入对话条目，MUST NOT 冒充 status、footer、通知条或队列条。折叠/展开/换行续行与上下轮廓（含 `↑ N more` / `↓ N more` 边框）MUST 计入 Fixed-Zone Footprint 实际行（可见行，含高度上限后的内容视口高加轮廓，不含未滚入内容）。

  @executable @req:r21
  场景: todo-bar-sits-between-queue-and-toast
    当 产品 TUI 同时存在非空待办栏、队列条与通知条
    那么 从上到下 MUST 为队列条、待办栏、通知条、status、editor
    并且 空表时待办栏 MUST 占 0 行
