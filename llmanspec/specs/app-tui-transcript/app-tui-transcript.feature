# language: zh-CN
# capability: app-tui-transcript
# purpose: Live 输出进引擎 scrollback（非 Codex 式 transcript 浏览面）；分支回看走会话树。
# scope: 产品 TUI 面

功能: app-tui-transcript

  @req:att1 @human
  场景: live-scrollback-output
    - 产品 UiRoot 当前轮 live 用户/助手/系统输出 MUST 写入引擎 scrollback 并以包 Markdown（及角色色/glyph）呈现；MUST NOT 截断历史冒充滚动；MUST NOT 以此要求实现 Codex 式可导航 transcript 浏览面。

  @req:att3 @human
  场景: diff-via-package
    - 产品 live scrollback 中文件编辑 diff MUST 复用 packages/xylitol-tui Diff（含 word-level 与复制友好前缀）；MUST NOT 在应用面复制准通用 diff 渲染器。

  @req:att4 @human
  场景: tool-block-status-rail
    - 产品默认以 rail 皮肤绘制 tool 可展开块：每可见内容行左侧 MUST 为 1 列 status 色轨 + 1 列无底色 gutter，再跟内容；pending/success/error 轨色 MUST 对齐 DESIGN accent/success/error（MAY soft-mix surface）。MUST NOT 默认整行铺 tool-*-bg。edit/Diff（含 unified display_diff）的 header 与 Diff 正文 MUST 共用同一 status 轨色（无洗底信封）；正文 MUST NOT 再叠 diff-*-bg 行底（极性靠 fg + word_wash_bg）。外层 ANSI 背景 MUST 以 \\x1b[49m 复位。

  @req:att5 @human
  场景: tool-rail-via-package-helper
    - rail 行绘制 MUST 复用 packages/xylitol-tui 的 paint_left_rail_line（或等价包 API：轨+gutter+内容宽）；MUST NOT 在应用面手写第二套轨/gutter/宽预算逻辑。

  @req:att6 @human @manual
  场景: no-codex-transcript-view
    - 产品 TUI MUST NOT 实现 Codex 风格的独立 transcript 浏览面或专用 TranscriptView 作为主 UX；历史/分支 travel MUST 经双 Esc 会话树（app-tui session-tree / package TreeSelector）。

  @req:att7 @human
  场景: expandable-block-fold
    - 产品 live scrollback 的 thinking/tool/edit-diff 可展开块 MUST 支持折叠与展开；折叠态 MUST 保留可读摘要行；状态机 MUST 在应用面；展开快捷键旁注 MUST 为括号完整和弦（Ctrl+T / Alt+E）。

  @req:att8 @human
  场景: expandable-key-hints
    - 可展开块旁 MUST 提示对应快捷键，格式为括号包裹的完整和弦（如 (Ctrl+T) 用于 thinking 块、(Alt+E) 用于 tool/diff）。thinking 块外显标签 MUST 为 Title Case：思考通道流式期间为 Thinking；思考通道结束后为 Thought，有可靠思考通道起止墙钟时 MUST 写 Thought {Ns}（如 Thought 17s）；起止 MUST 为思考开始到思考通道切走（先到的正文、工具意图或 ThinkingEnd），MUST NOT 计入随后正文或工具流；MUST NOT 在思考流式过程中每帧刷新时长。无 ActivityFold 时流式 thinking MUST 保持展开，结束后可折叠。agent_demo MUST 用 Ctrl+T 切换 thinking 展开态、Alt+E 同时切换 tool 与 diff 展开态。

  @req:att9 @human
  场景: product-bash-result-scrollback
    - 产品 TUI 在 idle bang 提交时 MUST 立即将命令写入 live scrollback 可着色块；执行中 MUST 为 pending 轨色，且输出 chunk 到达时 MUST 在同一块内增量刷新（保持 pending 轨）；完成后 MUST 按成功或失败或 cancelled 切换轨色；bang Esc 取消 MUST 在该块内呈现 (cancelled) 而非 agent 的 Aborted；exit_code 非 0 MUST 以 error 前景或 error 轨强调；MUST NOT 默认整行铺 tool-*-bg；MUST NOT 为此引入 Codex 式 TranscriptView。

  @req:att10 @human
  场景: scrollback-block-gap
    - 产品 live scrollback 相邻块之间 MUST 至少有一行 untinted 空行隔开（对齐 agent_demo Spacer）；MUST NOT 把块内容粘成连续墙；MUST NOT 要求块内 padding_y 空 tint 行或整行 tool-*-bg / user-message-bg 洗底。

  @req:att11 @human
  场景: bang-block-rail-via-helper
    - bang 块 rail 行 MUST 复用 paint_left_rail_line（或等价包 API）；MUST NOT 在应用面手写第二套轨绘制；MUST NOT 默认整行 apply_background_to_line 铺 tool-*-bg。

  @req:att12 @human
  场景: history-rebuild-from-parts
    - 产品 TUI 从 SessionEntry 重建 live scrollback（travel / fork / resume 共用路径）时 MUST 按 message.content 的 typed 部件投影：type=thinking → UiEntry::Thinking；type=text → UiEntry::Assistant（或等价助手正文）；toolCall → UiEntry::Tool（id=toolCallId）。同一路径上的 toolResult MUST 按 toolCallId 合入已有 Tool 行（填 output / is_error / done / display_diff 等，对齐直播 ToolExecutionEnd），使得同一次工具调用在 scrollback 中恰好一条 UiEntry::Tool；MUST NOT 在可配对时另起以 session entry id 为 id、缺 args_preview 的第二条 Tool。缺匹配 call 的 orphan toolResult MUST 仍可投影为单行 done Tool。MUST NOT 仅用 message_text 把 thinking+text 拼成单一 Assistant。同一条 JSONL 重建结果 MUST 与该轮直播 flush_streaming / ToolExecutionEnd 后的 Thinking/Assistant/Tool 分块形态幂等（可测）。

  @req:att13 @human
  场景: tool-human-summary
    - 折叠态 tool 块 args_preview MUST 使用人类可读的位置摘要（工具名由 header 单独绘制，摘要 MUST NOT 携带工具名前缀）：bash/shell → `$ {command}`；read/ls/edit/write 等取路径槽（path 键认 path/file_path/file 及 edits[0]）；read 有 offset/limit 时 MUST 附 `:start` 或 `:start-end`；edit 摘要 MUST NOT 附加行域（行号在 diff 正文）。path 缺失时 MUST 用 `...` 占位，MUST NOT 把完整 args JSON（含 content/edits/oldText/newText）当作默认 args_preview。历史重建 MUST 与直播路径共用同一摘要 helper。

  @req:att14 @human
  场景: write-edit-process-chrome
    - 产品 write 工具块 MUST 在意图/执行过程中默认展示正文 viewport（对齐流式跟尾：默认至多 10 逻辑行 Tail，超出 MUST 提示 earlier lines 与 ctrl+o）；write 的 header 与正文 MUST 共用同一 status 轨色（pending/success/error），MUST NOT 默认整行 tool-*-bg 洗底，MUST NOT 仅头行有轨而正文裸露无轨。edit 成功后 diff MUST 在同一工具块内默认可见（MUST NOT 依赖 Alt+E 才露出）。工具头行 MUST NOT 嵌入 [ok]/[err]/[…] 字面状态标签；成败 MUST 由轨色与块末错误行表达。

  @req:att15 @human
  场景: bash-full-output-footer-chrome
    - 产品 live scrollback 中 bang Bash 块与 bash 工具块的输出若含以 `[Full output:` 开头的脚注行，该行 MUST 以 DESIGN `{colors.warning}` 前景绘制（可 bold）；截断时 MUST 展示该脚注而非仅用 `(truncated)` 字面替换；未截断 MUST NOT 伪造该脚注。

  @req:att16 @human
  场景: hard-truncated-no-viewport-expand
    - 产品 bang Bash 与 tool 块（含 bash）的输出若含 `[Full output:`（系统硬截断），Ctrl+O 视口 MUST NOT 展开为全文；渲染 MUST 保持 Tail 预览并提示 expand disabled（或等价）；write 正文 viewport MUST 仍允许 Ctrl+O 展开。ToolExecutionEnd 在 truncated 时 MUST 用截断后的 combined/stdout 替换流式累积的 output 缓冲。

  @req:att17 @human
  场景: diff-render-cap
    - 产品 live scrollback 渲染 edit/Diff 的 display_diff 时 MUST 限制可见行数（硬上限）；超限 MUST 截断并提示 omitted；过大 diff MUST NOT 启用 word-level，以免卡死 TUI。

  @req:att18 @human
  场景: no-prepend-nav-notices
    - 产品 TUI 在 travel/fork/resume/切换等重建或瞬时导航通知写入 live scrollback 时，MUST 将此类 ScrollNotice/Error 滚动提示追加到 entries 末尾（跟底时位于输入框上方可滚区域）；MUST NOT 为提高可见性而把瞬时导航通知 prepend 到 entries 前缀。会话路径上的时间线内容（含 BranchSummary 等按祖先投影的条目）不在本条「瞬时通知」范围。

  @req:att19 @human
  场景: fold-glyph-chevron
    - 产品折叠标记 Unicode 字形 MUST 为收起可展开 ▸、展开可收起 ▾；Ascii 回退 MUST 为 > / v；标记可视宽 MUST 为 1 列；MUST 尊重 XYLITOL_TUI_GLYPH_SET=ascii。本要求不规定 Bash/Compaction 等无三角块。

  @req:att20 @human
  场景: per-block-tools-fold-overrides
    - 产品对 Tool、独立 Diff、Ask 可展开块 MUST 以 tools 族默认展开态加 per-block 覆盖表决定有效展开态（覆盖优先于默认）。左键单击折叠三角列 MUST 仅翻转该块覆盖（或等价单块有效态），MUST NOT 改变其它块的覆盖。Alt+E MUST 翻转 tools 族默认展开态并清空该族全部覆盖。本要求不拆 Alt+E 与 compaction 的既有连带（若有）。单块态变 MUST 遵守 ath25：禁止因此对全部历史 Assistant 重做 Markdown 解析。

  @req:att21 @human
  场景: thinking-per-id-fold
    - 产品 Thinking 块 MUST 支持 per-id 展开态（默认 thinking 展开态加 per-id 覆盖，覆盖优先）；每条 Thinking MUST 有稳定 id，且 live 与 travel/fork/resume 重建路径 MUST 对同一逻辑块使用同一 id。左键单击该块折叠三角列 MUST 仅翻转该 Thinking 的覆盖。Ctrl+T MUST 翻转 thinking 默认展开态并清空全部 thinking 覆盖。思考通道流式期间外显 MUST 为 Thinking；思考通道结束后 MUST 为 Thought（有可靠思考通道起止墙钟则 Thought {Ns}）。resume / rebuild MUST 优先用落地的 thinkingElapsedSecs；否则用落盘思考通道起止节点 unix-ms 相减；缺任一端 MUST 省略时长，MUST NOT 用相邻条目墙钟冒充思考时长，禁止伪造。无 ActivityFold 时流式 thinking MUST 保持展开（与 att8 一致），结束后可按有效态折叠。

  @req:att22 @human
  场景: mouse-fold-triangle-column-only
    - 产品在 ApplicationOwned 下，对 att20/att21 范围内块：折叠命中 MUST 仅覆盖折叠三角列（单列可视宽）；点击摘要正文、旁注和弦或其它非三角列 MUST NOT 因此 toggle 折叠。transcript 拖选进行中 MUST 忽略折叠命中（不因划过三角而 toggle）。本要求不涵盖信封/簇头行；Ask 等待时 Asking questions 整行可点见 att31 与 att33。

  @req:att23 @human
  场景: activity-nested-envelope-cluster
    - 产品 MUST 将 live scrollback 中间活动按嵌套 Activity 管理：信封（一轮）套簇套块。信封折叠可见 MUST 为 User + 一行 Worked for + 该轮最后一段助手正文；中间助手正文与 Todo、Compaction、工具与 thinking 一并收纳。ScrollNotice 与 Error MUST NOT 进信封。信封展开 MUST 仍画出夹在簇之间的中间助手正文，且 MUST 留下 Worked for 头行与展开标记以便再折。簇展开后块级折叠 MUST 仍服从 att20/att21。每信封与每簇 MUST 有稳定 id，且 live 与 travel/fork/resume 重建 MUST 同构。展开任一级 MUST 留下该级头行与展开标记（att19），以便再折；例外：一簇的中间活动全部为 Compaction 时 MUST NOT 再画簇摘要头，信封展开 MUST 直接露出既有 Compaction 块。信封头、簇头与块头 MUST 与未折叠细账同一列展示；MUST NOT 用前导缩进表达嵌套层级。

  @req:att24 @human
  场景: cluster-summary-envelope-worked-for
    - 簇摘要行 MUST 只描述真实发生的活动，英文 Title Case。文件层 MUST 互斥：有改写（edit/write 等）则只写 Edited；否则有读或搜索则写 Explored；MUST NOT 同一头并列 Edited 与 explored。有 shell 才追加 Ran N commands；仅 shell、无文件活动时整行 MUST 为 Ran。N MUST 为去重 path；仅一个 path 时 MUST 写可用 basename（`.` / `..` MUST NOT 当文件名，可回退上一路径分量；仍无可用名则写 N file，MUST NOT 写 Edited . / Explored .）；多个 MUST 写 N files。无 path 的搜索 MUST NOT 加成假文件数，但仍可使该簇进入 Explored。混合簇头 MUST NOT 写入 thinking、MCP 或 compaction。当文件层与 Ran 皆空：仅 thinking 流式未结束 MUST 为 Thinking（无时长）；思考通道结束后 MUST 为 Thought，有可靠思考通道起止墙钟才附 Thought {Ns}；resume MUST 优先用落地的 thinkingElapsedSecs 恢复 Thought {Ns}；否则用落盘思考通道起止节点 unix-ms 相减；缺任一端 MUST 省略时长，MUST NOT 用相邻条目墙钟冒充思考时长，禁止伪造；MUST NOT 流式滴答时长。同一轮中途 thinking 不切簇（att34），与工具或 Ask 同簇时簇头 MUST 走 Edited / Explored / Ran / Used / Asking questions 等真实活动，MUST NOT 用 Thought 当聚合头；todo_* 与其它未知/MCP 工具一样走 Used，不单开类目。Used 的 N MUST 为调用次数（与 Ran 同构），同一工具多次调用 MUST 按次累加；仅一次时 MUST 写 Used {短名}；多次 MUST 写 Used N tools。checklist 投影行 MUST NOT 计入 Used N。仅 MCP 或未知工具 MUST 为 Used；仅 Ask MUST 为 Asking questions；仅 Compaction MUST NOT 画簇头（见 att23）。MUST NOT 用文件占位虚构 Explored。打开簇进行中 MUST 用 Editing / Exploring / Running（与文件互斥及 Ran 对齐）；封口后 MUST 用 Edited / Explored / Ran。类目计数 MUST 在工具开始时更新；+/- MUST 仅在有可靠 diff 统计时于工具结束附加，否则省略；MUST NOT 伪造 +/-。无中间操作 MUST NOT 生成空簇。信封 Worked for MUST 以可靠双端墙钟书写时长；缺任一端 MUST 仍可画 Worked for 但省略时长数字；MUST NOT 伪造时长。

  @req:att25 @human
  场景: activity-fold-layered-with-l1
    - 信封或簇折叠时：被收纳的内层块 MUST NOT 进入渲染；Alt+E、Ctrl+T、Ctrl+O 与 per-id 块级覆盖 MUST NOT 改变该折叠外观；被收纳块的折叠三角 MUST NOT 登记。信封与簇均展开时：既有块级默认态、覆盖表与三角命中 MUST 照常生效。折叠态切换 MUST 遵守 ath25：禁止因此对全部历史助手正文重做 Markdown 解析。

  @req:att26 @human
  场景: activity-fold-auto-collapse
    - 产品 MUST 提供 ActivityFold 自动收纳：enabled 默认 true（关闭则全细账，块级折叠仍可用）；keep_recent_turns 默认 2（仅约束 auto_on_turn_end：最近 K 个已结束 Activity 轮信封默认展开，更旧信封默认折叠）；auto_on_rebuild 默认 true（travel/resume/fork 重建后对全部已结束 Activity 轮套折叠信封，不按 keep_recent_turns 留近窗）；auto_on_turn_end 默认 true（回合结束后对超窗已结束轮套折叠）。流式当前轮 MUST NOT 被收成 Worked for 信封，MUST 走 att33 live window。YAML 键与非法值见 runtime-config。近窗口内从未进入 ActivityFold 的细账 MUST NOT 被当作 collapseNearest 目标。旧键 auto_l3_distant MUST NOT 再作为产品配置面。

  @req:att27 @human
  场景: activity-fold-markers-and-hints
    - 信封与簇头行折叠标记 MUST 遵守 att19（Unicode ▸/▾；Ascii >/v）。旁注 MUST 为当前绑定的完整和弦括号；默认展开旁注 MUST 对应 Alt+Shift+E，默认收纳旁注 MUST 对应 Ctrl+Alt+Shift+E；改绑后旁注 MUST 跟随当前绑定。信封/簇头 MUST NOT 写 (Alt+E)，以免与块级展开混淆。Ask 等待的 Asking questions MUST NOT 画折叠三角。MUST NOT 画 Planning next moves。

  @req:att28 @human
  场景: activity-expand-collapse-nearest
    - 产品 MUST 提供跨面动作语义 activity.expandNearest 与 activity.collapseNearest（TUI 键位 id 为 app.activity.expandNearest / app.activity.collapseNearest；默认和弦 Alt+Shift+E / Ctrl+Alt+Shift+E，可改绑）。expandNearest MUST 将距输入最近的折叠信封先展开；若该信封已展开则展开最近折叠簇。collapseNearest MUST 将距输入最近的展开簇先折；若无则折最近展开信封。无合格目标时两动作 MUST 静默（无报错、无 toast、无态变）。启发式 MUST 按 entries 序靠近输入；MUST NOT 抢 Editor 常驻焦点。一对动作 MUST 覆盖嵌套一级步进；MUST NOT 另开信封/簇专用默认键；弱终端 MUST NOT 另发官方第二默认和弦。

  @req:att29 @human
  场景: compaction-fold-triangle-mouse
    - 产品在 ApplicationOwned 下，Compaction 可折块 MUST 提供折叠三角列（字形遵守 att19）。左键单击该三角列 MUST 仅翻转全局 compaction 展开态（compaction_expanded 或等价）；MUST NOT 清空 tools 族覆盖，MUST NOT 翻转 tools 族默认展开态。单击摘要正文、旁注和弦或其它非三角列 MUST NOT 因此 toggle。键盘 Alt+E 与 compaction 的既有连带（若有）不在本条拆改范围。态变 MUST 遵守 ath25：禁止因此对全部历史 Assistant 重做 Markdown 解析。

  @req:att30 @human
  场景: output-viewport-hint-mouse
    - 产品在 ApplicationOwned 下，对渲染 Ctrl+O 输出视口 expand/collapse hint 的块（含 Tool、Bash、独立 Diff 等凡展示该 hint 者）：左键单击可见 hint 带 MUST 翻转全局 tools_output_expanded（或等价），与键盘 Ctrl+O 同构。命中几何 MUST 为 hint 文案可点带（非折叠三角列；att22 三角列-only 不适用本条）。Bash MUST NOT 另补块级 L1 折叠三角；其输出高度交互 MUST 走本条 Viewport。硬截断禁展开（att16）时 MUST NOT 因点击 hint 而展开全文。态变 MUST 遵守 ath25。

  @req:att31 @human
  场景: envelope-cluster-fold-marker-mouse
    - 产品在 ApplicationOwned 下，对信封与簇头行：左键单击折叠标记列（字形遵守 att19/att27；命中仅三角列）MUST 只 toggle 该级（信封或该簇），MUST NOT 使用 expandNearest/collapseNearest 启发式代替定点。单击摘要正文或旁注 MUST NOT 因此 toggle。对 Asking questions：整行可点 MUST 展开打开簇，属 att22 三角列-only 的例外。分层继承 att25：信封或簇折叠时 Alt+E、Ctrl+T、Ctrl+O 与 per-id 块级覆盖 MUST NOT 改变该折叠外观；被收纳块级折叠三角 MUST NOT 登记。态变 MUST 遵守 ath25。

  @req:att32 @human
  场景: remaining-fold-targets-unified-hit
    - 产品对 att29–att31 剩余可点折叠目标（Compaction、OutputViewport、信封、簇）MUST 登记进与 att20/att21 同一折叠命中表及 ath33 hit_priority 路径；MUST NOT 另开第二套命中管道。transcript 拖选进行中 MUST 忽略这些折叠命中（与 att22/ath33 同构）。本条不要求新增 host latch 管道（ath33 已覆盖吞按与拖选忽略）。

  @req:att33 @human
  场景: activity-live-window
    - 流式当前轮 MUST 使用 live window，MUST NOT 整轮收成 Worked for 信封。MUST NOT 画 Planning next moves 占位行；尚无可展示助手正文且无 thinking/Ask/工具时 MUST NOT 画空助手气泡，busy 短词走状态条 Working 或 Running {name}。Thinking 流 MUST 画 Thinking 簇头（可点展开，默认折叠正文，无时长）；思考通道结束后 MUST 更新为 Thought，有可靠思考通道起止墙钟 MUST 写 Thought {Ns}（如 Thought 17s）；起止 MUST 不含随后正文或工具流；已封 Thought 簇 MUST NOT 因后续 Thinking 流改回 Thinking；resume 优先落地 thinkingElapsedSecs，否则思考通道节点 unix-ms 相减，缺戳则省略时长；MUST NOT 用相邻条目墙钟冒充思考时长；MUST NOT 流式每帧刷新时长。仅 thinking 的簇 MUST NOT 同时画 Thinking/Thought 簇头与 thinking L1 头（合并为一行）。与工具同簇时簇头走 att24 真实活动，L1 仍可 Thought {Ns}。Ask 等待 MUST 显示 Asking questions，且 Ask 块 MUST 可交互、不得折没。打开簇有进行中或已结束的工具时 MUST 画带折叠三角的簇头（att24 进行时 Editing / Exploring / Running）；流式工具块 MUST 作为该簇子项，默认折叠，点簇头三角 MUST 可展开（含流式）。同一打开簇内工具结束或多次调用归并 MUST NOT 自动收起已展开的子项。paint MUST NOT 每帧改折叠态。MUST NOT 把进行中工具只画成无三角的 live 尾行而藏起块。MUST NOT 把无改写的读文件写成 Editing；MUST NOT 虚构 Explored。助手正文 MUST NOT 被收进簇。已封簇与更早内容 MUST NOT 因打开簇更新而重算。点 Asking questions MUST 揭开已算好的 sealed 细账。状态条短词 MUST 仍为 Working 或 Running {name}。已结束轮何时套 Worked for 信封见 att26，不在 live window 内提前套。

  @req:att34 @human
  场景: cluster-split-assistant-body
    - 簇边界 MUST 以已可展示的助手正文划分：正文第一个非空白字符 MUST 封口上一打开簇。Thinking、工具、Ask、Diff、Todo、Compaction MUST NOT 单独切簇。同一轮中途再思考仍留在打开簇内，簇头措辞见 att24。同一条目序下 live 与 resume/rebuild 切分 MUST 同构。

  @req:att19 @executable
  场景: fold-glyph-unicode-and-ascii-fallback
    假如 折叠字形环境未指定（默认 Unicode 集）
    当 读取折叠与展开字形
    那么 折叠为 ▸ 展开为 ▾ 且各占单列
    并且 切换环境变量 XYLITOL_TUI_GLYPH_SET=ascii 并重新读取
    并且 折叠回退为 > 展开回退为 v 且各占单列

  @req:att13 @executable
  场景: tool-human-summary-location-only
    当 折叠态读取 bash、read、write 三类参数人话摘要
    那么 bash 前缀 $ 且 read 附行号区间且 write 为纯路径不带名前缀
    并且 缺 path 时用三点占位且不回退完整 args JSON

  @req:att10 @executable
  场景: scrollback-block-gap-headless
    当 以场景构建器渲染相邻的助手块与工具块（宽 80）
    那么 相邻块之间至少一行空行分隔且不粘连成墙

  @req:att24 @executable
  场景: cluster-head-wording-exclusivity
    当 以场景构建器回放读后改写序列（read old.rs 然后 edit a.rs）
    那么 只读前簇封口为 Explored old.rs 且改写簇头保持 Editing a.rs
    并且 改写结束后无 Edited 错时态
    并且 全帧不出现 Worked for 与 Planning next moves

  @req:att34 @executable
  场景: assistant-body-seals-cluster-headless
    当 以场景构建器在两个工具活动之间插入助手正文
    那么 正文封口前簇且新簇在其下方独立开口

  @req:att33 @executable
  场景: live-window-unenveloped-headless
    当 以场景构建器渲染流式思考中的 live window
    那么 出现 Thinking 簇头且无 Thought 与 Ctrl+T 旁注
    当 以场景构建器渲染含助手正文的 live window
    那么 助手正文可见且仍无信封封套
    并且 全帧不出现 Worked for 与 Planning next moves

  @req:att20 @executable
  场景: tools-per-block-fold-click-and-alt-e-headless
    当 以场景构建器回放读后改写序列并封轮挂载交互面
    当 左键单击折叠命中表中的簇头三角列
    当 再左键单击 "t-read" 的工具块三角列
    那么 该块经覆盖表收起且覆盖表只有这一个条目
    当 按下和弦 Alt+E
    那么 工具族默认展开态翻转且块级覆盖清空

  @req:att21 @executable
  场景: thinking-per-id-fold-click-and-ctrl-t-headless
    当 以场景构建器回放思考加工具并封轮挂载交互面
    当 左键单击折叠命中表中的簇头三角列
    当 再左键单击第一条思考的折叠三角列
    那么 该条思考经覆盖展开且另一条不受影响
    当 按下和弦 Ctrl+T
    那么 思考默认展开态翻转且全部 per-id 覆盖清空

  @req:att22 @executable
  场景: mouse-fold-hit-triangle-column-only-headless
    当 以场景构建器回放读后改写序列并封轮挂载交互面
    当 左键单击折叠命中表中的簇头三角列
    当 点击该行折叠命中区右边界之外的一列
    那么 折叠命中不消费且覆盖表为空
    当 再左键单击 "t-edit" 的工具块三角列
    那么 该块经覆盖表收起且覆盖表只有这一个条目

  @req:att25 @executable
  场景: activity-fold-layered-collapse-hides-blocks-headless
    当 以场景构建器回放读后改写序列并封轮挂载交互面
    那么 簇即为折叠收纳态且无内层块登记
    当 依序按下和弦 Alt+E、Ctrl+T、Ctrl+O
    那么 渲染帧与按键前逐字一致
    当 左键单击折叠命中表中的簇头三角列
    那么 仅该簇被定点展开且内层块行可见

  @req:att31 @executable
  场景: cluster-fold-marker-mouse-precise-headless
    当 以场景构建器回放读后改写序列并封轮挂载交互面
    当 点击簇头行的摘要正文列而非三角列
    那么 折叠命中不消费且覆盖表为空
    当 左键单击折叠命中表中的簇头三角列
    那么 仅该簇被定点展开且内层块行可见

  @req:att29 @executable
  场景: compaction-fold-triangle-mouse-headless
    当 以场景构建器回放压缩事件并封轮挂载交互面
    当 左键单击 Compaction 块的折叠三角列
    那么 Compaction 全局展开态翻转
    并且 工具族覆盖表仍为空且工具族默认态未变

  @req:att30 @executable
  场景: output-viewport-hint-band-mouse-headless
    当 以场景构建器回放多行输出的工具并封轮挂载交互面
    当 左键单击折叠命中表中的簇头三角列
    当 点击输出的 Ctrl+O 提示带
    那么 提示带点击翻转输出视口全局态且与按 Ctrl+O 同构

  @req:att32 @executable
  场景: remaining-fold-targets-unified-hit-table-headless
    当 以场景构建器回放压缩加多行输出并封轮挂载交互面
    当 左键单击折叠命中表中的簇头三角列
    那么 Compaction、输出提示带与块级三角登记于同一命中表且定点可点

  @req:att7 @executable
  场景: expandable-block-fold-summary-and-chords-headless
    当 以场景构建器回放读后改写序列并封轮挂载交互面
    当 左键单击折叠命中表中的簇头三角列
    当 再左键单击 "t-read" 的工具块三角列
    那么 收起块保留摘要行且展开旁注为括号完整和弦

  @req:att1 @executable
  场景: live-scrollback-no-truncation-headless
    当 以场景构建器渲染多段助手正文（宽 80）
    那么 各段正文均在帧内且早段未被挤出
    并且 助手正文行携带样式转义

  @req:att8 @executable
  场景: thought-label-duration-and-hint-headless
    当 以场景构建器回放思考加工具并结算 7 秒封轮挂载交互面
    当 左键单击折叠命中表中的簇头三角列
    那么 结算行外显 Thought 7s 且不再出现流式 Thinking 头
    并且 思考块旁注为括号完整和弦 Ctrl+T

  @req:att12 @executable
  场景: history-rebuild-tool-merge-headless
    当 以含思考正文与同 id 工具调用加结果的条目重建 transcript
    那么 思考正文工具各成一块且工具恰一行不另起第二工具
    并且 重建工具行与同轮直播工具行逐字段一致

  @req:att18 @executable
  场景: travel-notice-trailing-append-headless
    当 重建到叶节点并按产品路径追加路径通告
    那么 通告条目位于 entries 末尾且重建内容次序保持原样

  @req:att4 @executable
  场景: tool-rail-status-colors-headless
    当 以场景构建器回放 pending、成功与失败三种工具并取 ANSI 帧
    那么 pending 轨用 accent 而成功轨用 success 且失败轨用 error
    并且 轨为单列加无底色 gutter 且外层背景以复位码收束
    并且 内容行除轨外无整行洗底

  @req:att14 @executable
  场景: write-viewport-and-edit-diff-chrome-headless
    当 以场景构建器回放超长 write 正文并挂载交互面
    那么 正文视口至多 10 行尾且以 total 加 ctrl+o 提示省略
    并且 write 头行与正文共用同一状态轨
    当 以场景构建器回放 edit 成功并挂载交互面
    那么 diff 正文默认可见且头行无状态字面标签

  @req:att15 @executable
  场景: bash-full-output-footer-warning-headless
    当 以场景构建器回放带 Full output 脚注的 bash 工具并取 ANSI 帧
    那么 脚注行以 warning 前景绘制且帧内可见脚注
    当 以场景构建器回放未截断的正常输出
    那么 帧内不出现伪造的 Full output 脚注

  @req:att16 @executable
  场景: hard-truncated-viewport-guard-headless
    当 以场景构建器回放硬截断 bash 工具并按下 Ctrl+O
    那么 视口保持尾窗且提示 expand disabled 且不出全文
    当 回放超长 write 正文并按下 Ctrl+O
    那么 write 正文可展开为全文

  @req:att23 @executable
  场景: envelope-fold-hides-inner-keeps-heads-headless
    当 以场景构建器回放四轮活动并按回合结束收纳
    那么 最旧信封折叠为用户行加 Worked for 加末段正文且无内层块
    当 左键单击折叠命中表中的信封三角列
    那么 该信封定点展开且中间助手正文与簇头行重新可见

  @req:att26 @executable
  场景: activity-auto-collapse-windows-headless
    当 以场景构建器回放四轮活动并按回合结束收纳
    那么 仅最旧两轮折叠为 Worked for 且近窗轮正文与簇头保持
    当 以重建路径应用重建收纳
    那么 全部四轮折叠为 Worked for
    当 关闭 ActivityFold 重挂并按回合结束收纳
    那么 全帧无 Worked for

  @req:att27 @executable
  场景: envelope-cluster-markers-and-chords-headless
    当 以场景构建器回放四轮活动并按回合结束收纳
    那么 折叠信封与收起簇头旁注均为 Alt+Shift+E
    当 左键单击折叠命中表中的簇头三角列
    那么 展开簇头旁注为 Ctrl+Alt+Shift+E 且信封簇头行不带 (Alt+E)
    并且 全帧无 Planning next moves

  @req:att28 @executable
  场景: activity-expand-collapse-nearest-keys-headless
    当 以场景构建器回放四轮活动并按重建收纳
    当 按下和弦 Alt+Shift+E
    那么 最近信封降为簇头态且 Worked for 保持
    当 再按下和弦 Alt+Shift+E
    那么 次近折叠信封降为簇头态且最近簇头保持
    当 按下和弦 Ctrl+Alt+Shift+E
    那么 最近展开信封收回为 Worked for 折叠态
    当 再按下和弦 Ctrl+Alt+Shift+E
    那么 全部信封收回为 Worked for 折叠态

  @req:att9 @executable
  场景: product-bash-block-lifecycle-headless
    当 以主机泵提交 bang 命令并注入分段输出与成功结果
    那么 命令进入单一 Bash 块且输出在同一块内且状态轨为成功
    当 注入取消的结果
    那么 块内呈现 (cancelled) 而非 agent 的 Aborted
    当 注入非零退出码结果
    那么 块状态为 error 且以错误轨强调

  @req:att11 @executable
  场景: bang-block-rail-no-wash-headless
    当 以主机泵提交 bang 命令并完成成功结果
    那么 bang 块行带单列状态轨加无底色 gutter 且内容区无整行洗底
