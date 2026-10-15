# language: zh-CN
# purpose: Live 输出进引擎 scrollback（非 Codex 式 transcript 浏览面）；分支回看走会话树。

功能: app-tui-transcript

  @req:r1339
  规则: live-scrollback-output
    产品 UiRoot 当前轮 live 用户/助手/系统输出 须写入引擎 scrollback 并以包 Markdown（及角色色/glyph）呈现；不得截断历史冒充滚动；不得以此要求实现 Codex 式可导航 transcript 浏览面。

    场景: live-scrollback-no-truncation-headless
      当 以场景构建器渲染多段助手正文（宽 80）
      那么 各段正文均在帧内且早段未被挤出
      并且 助手正文行携带样式转义
  @req:r1360
  规则: diff-via-package
    产品 live scrollback 中文件编辑 diff 须复用 packages/xylitol-tui Diff（含 word-level 与复制友好前缀）；不得在应用面复制准通用 diff 渲染器。

    场景: diff-reuses-package-component
      当 读取应用面对包组件的复用
      那么 diff 渲染取自包的 Diff 且无第二套
    # verified-by: src/app/tui/widgets/scrollback/diff.rs
  @req:r1368
  规则: tool-block-status-rail
    产品默认以 rail 皮肤绘制 tool 可展开块：每可见内容行左侧 须为 1 列 status 色轨 + 1 列无底色 gutter，再跟内容；pending/success/error 轨色 须对齐 DESIGN accent/success/error（MAY soft-mix surface）。不得默认整行铺 tool-*-bg。edit/Diff（含 unified display_diff）的 header 与 Diff 正文 须共用同一 status 轨色（无洗底信封）；正文 不得再叠 diff-*-bg 行底（极性靠 fg + word_wash_bg）。外层 ANSI 背景 须以 \\x1b[49m 复位。

    场景: tool-rail-status-colors-headless
      当 以场景构建器回放 pending、成功与失败三种工具并取 ANSI 帧
      那么 pending 轨用 accent 而成功轨用 success 且失败轨用 error
      并且 轨为单列加无底色 gutter 且外层背景以复位码收束
      并且 内容行除轨外无整行洗底
  @req:r1369
  规则: tool-rail-via-package-helper
    rail 行绘制 须复用 packages/xylitol-tui 的 paint_left_rail_line（或等价包 API：轨+gutter+内容宽）；不得在应用面手写第二套轨/gutter/宽预算逻辑。

    场景: left-rail-reuses-package-painter
      当 读取应用面对包组件的复用
      那么 左轨只经包 paint_left_rail_line 绘制
    # verified-by: packages/xylitol-tui/src/theme/paint.rs
  @req:r1370
  规则: no-codex-transcript-view
    产品 TUI 不得实现 Codex 风格的独立 transcript 浏览面或专用 TranscriptView 作为主 UX；历史/分支 travel 须经双 Esc 会话树（app-tui session-tree / package TreeSelector）。

    场景: travel-via-double-esc-tree
      当 以主机泵空编辑器双 Esc 开树
      那么 树槽渲染 Search 行与 TreeHelp 行
    # verified-by: src/AGENTS.md
  @req:r1371
  规则: expandable-block-fold
    产品 live scrollback 的 thinking/tool/edit-diff 可展开块 须支持折叠与展开；折叠态 须保留可读摘要行；状态机 须在应用面；展开快捷键旁注 须为括号完整和弦（Ctrl+T / Alt+E）。

    场景: expandable-block-fold-summary-and-chords-headless
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      当 再左键单击 "t-read" 的工具块三角列
      那么 收起块保留摘要行且展开旁注为括号完整和弦
  @req:r1372
  规则: expandable-key-hints
    可展开块旁 须提示对应快捷键，格式为括号包裹的完整和弦（如 (Ctrl+T) 用于 thinking 块、(Alt+E) 用于 tool/diff）。thinking 块外显标签 须为 Title Case：思考通道流式期间为 Thinking；思考通道结束后为 Thought，有可靠思考通道起止墙钟时 须写 Thought {Ns}（如 Thought 17s）；起止 须为思考开始到思考通道切走（先到的正文、工具意图或 ThinkingEnd），不得计入随后正文或工具流；不得在思考流式过程中每帧刷新时长。无 ActivityFold 时流式 thinking 须保持展开，结束后可折叠。agent_demo 须用 Ctrl+T 切换 thinking 展开态、Alt+E 同时切换 tool 与 diff 展开态。

    场景: thought-label-duration-and-hint-headless
      当 以场景构建器回放思考加工具并结算 7 秒封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      那么 结算行外显 Thought 7s 且不再出现流式 Thinking 头
      并且 思考块旁注为括号完整和弦 Ctrl+T
  @req:r1373
  规则: product-bash-result-scrollback
    产品 TUI 在 idle bang 提交时 须立即将命令写入 live scrollback 可着色块；执行中 须为 pending 轨色，且输出事件（session/bash_output）到达时 须在同一块内增量刷新（保持 pending 轨）；完成后 须按成功或失败或 cancelled 切换轨色；resume 重建时，同一 bash_id 有 done 结果 须只显示 done（不得重复），仅 running 无 done 的 须组合渲染为 interrupted（命令 + 中断提示，全文路径若在）；bang Esc 取消 须在该块内呈现 (cancelled) 而非 agent 的 Aborted；exit_code 非 0 须以 error 前景或 error 轨强调；不得默认整行铺 tool-*-bg；不得为此引入 Codex 式 TranscriptView。

    场景: product-bash-block-lifecycle-headless
      当 以主机泵提交 bang 命令并注入分段输出与成功结果
      那么 命令进入单一 Bash 块且输出在同一块内且状态轨为成功
      当 注入取消的结果
      那么 块内呈现 (cancelled) 而非 agent 的 Aborted
      当 注入非零退出码结果
      那么 块状态为 error 且以错误轨强调
  @req:r1340
  规则: scrollback-block-gap
    产品 live scrollback 相邻块之间 须至少有一行 untinted 空行隔开（对齐 agent_demo Spacer）；不得把块内容粘成连续墙；不得要求块内 padding_y 空 tint 行或整行 tool-*-bg / user-message-bg 洗底。

    场景: scrollback-block-gap-headless
      当 以场景构建器渲染相邻的助手块与工具块（宽 80）
      那么 相邻块之间至少一行空行分隔且不粘连成墙
  @req:r1341
  规则: bang-block-rail-via-helper
    bang 块 rail 行 须复用 paint_left_rail_line（或等价包 API）；不得在应用面手写第二套轨绘制；不得默认整行 apply_background_to_line 铺 tool-*-bg。

    场景: bang-block-rail-no-wash-headless
      当 以主机泵提交 bang 命令并完成成功结果
      那么 bang 块行带单列状态轨加无底色 gutter 且内容区无整行洗底
  @req:r1342
  规则: history-rebuild-from-parts
    产品 TUI 从 SessionEntry 重建 live scrollback（travel / fork / resume 共用路径）时 须按 message.content 的 typed 部件投影：type=thinking → UiEntry::Thinking；type=text → UiEntry::Assistant（或等价助手正文）；toolCall → UiEntry::Tool（id=toolCallId）。同一路径上的 toolResult 须按 toolCallId 合入已有 Tool 行（填 output / is_error / done / display_diff 等，对齐直播 ToolExecutionEnd），使得同一次工具调用在 scrollback 中恰好一条 UiEntry::Tool；不得在可配对时另起以 session entry id 为 id、缺 args_preview 的第二条 Tool。缺匹配 call 的 orphan toolResult 须仍可投影为单行 done Tool。不得仅用 message_text 把 thinking+text 拼成单一 Assistant。同一条 JSONL 重建结果 须与该轮直播 flush_streaming / ToolExecutionEnd 后的 Thinking/Assistant/Tool 分块形态幂等（可测）。

    场景: history-rebuild-tool-merge-headless
      当 以含思考正文与同 id 工具调用加结果的条目重建 transcript
      那么 思考正文工具各成一块且工具恰一行不另起第二工具
      并且 重建工具行与同轮直播工具行逐字段一致
  @req:r1343
  规则: tool-human-summary
    折叠态 tool 块 args_preview 须使用人类可读的位置摘要（工具名由 header 单独绘制，摘要 不得携带工具名前缀）：bash/shell → `$ {command}`；read/ls/edit/write 等取路径槽（path 键认 path/file_path/file 及 edits[0]）；read 有 offset/limit 时 须附 `:start` 或 `:start-end`；edit 摘要 不得附加行域（行号在 diff 正文）；todo_* → 按工具：todo_rewrite 有全表 `items` 时为条目计数摘要（如 `5 items · 1 in progress`）；todo_update 有补丁 `items` 时为 `{n} updated`（n 为数组长度）；不得展开完整 items JSON。rewrite/update 流式未齐无 `items` 时摘要 须为空串（header 只画工具名），不得回落 `...` 占位（`...` 是路径槽占位，仅限路径族工具）。path 缺失时 须用 `...` 占位，不得把完整 args JSON（含 content/edits/oldText/newText）当作默认 args_preview。历史重建 须与直播路径共用同一摘要 helper。

    场景: tool-human-summary-location-only
      当 折叠态读取 bash、read、write 三类参数人话摘要
      那么 bash 前缀 $ 且 read 附行号区间且 write 为纯路径不带名前缀
      并且 缺 path 时用三点占位且不回退完整 args JSON

    场景: todo-summary-empty-not-placeholder
      当 折叠态读取无 items 的 todo_rewrite 与带补丁 items 的 todo_update 的人话摘要
      那么 todo_rewrite 摘要 为空串（header 只画工具名）
      并且 todo_update 摘要 匹配补丁计数（如 `1 updated`），不为空串或三点占位
  @req:r1344
  规则: write-edit-process-fixed-zone
    产品 write 工具块 须在意图/执行过程中默认展示正文 viewport（对齐流式跟尾：默认至多 10 逻辑行 Tail，超出 须提示 earlier lines 与 ctrl+o）；write 的 header 与正文 须共用同一 status 轨色（pending/success/error），不得默认整行 tool-*-bg 洗底，不得仅头行有轨而正文裸露无轨。edit 成功后 diff 须在同一工具块内默认可见（不得依赖 Alt+E 才露出）。工具头行 不得嵌入 [ok]/[err]/[…] 字面状态标签；成败 须由轨色与块末错误行表达。

    场景: write-viewport-and-edit-diff-fixed-zone-headless
      当 以场景构建器回放超长 write 正文并挂载交互面
      那么 正文视口至多 10 行尾且以 total 加 ctrl+o 提示省略
      并且 write 头行与正文共用同一状态轨
      当 以场景构建器回放 edit 成功并挂载交互面
      那么 diff 正文默认可见且头行无状态字面标签
  @req:r1345
  规则: bash-full-output-footer-fixed-zone
    产品 live scrollback 中 bang Bash 块与 bash 工具块的输出若含以 `[Full output:` 开头的脚注行，该行 须以 DESIGN `{colors.warning}` 前景绘制（可 bold）；截断时 须展示该脚注而非仅用 `(truncated)` 字面替换；未截断 不得伪造该脚注。

    场景: bash-full-output-footer-warning-headless
      当 以场景构建器回放带 Full output 脚注的 bash 工具并取 ANSI 帧
      那么 脚注行以 warning 前景绘制且帧内可见脚注
      当 以场景构建器回放未截断的正常输出
      那么 帧内不出现伪造的 Full output 脚注
  @req:r1346
  规则: hard-truncated-no-viewport-expand
    产品 bang Bash 与 tool 块（含 bash）的输出若含 `[Full output:`（系统硬截断），Ctrl+O 视口 不得展开为全文；渲染 须保持 Tail 预览并提示 expand disabled（或等价）；write 正文 viewport 须仍允许 Ctrl+O 展开。ToolExecutionEnd 在 truncated 时 须用截断后的 combined/stdout 替换流式累积的 output 缓冲。

    场景: hard-truncated-viewport-guard-headless
      当 以场景构建器回放硬截断 bash 工具并按下 Ctrl+O
      那么 视口保持尾窗且提示 expand disabled 且不出全文
      当 回放超长 write 正文并按下 Ctrl+O
      那么 write 正文可展开为全文
  @req:r1347
  规则: diff-render-cap
    产品 live scrollback 渲染 edit/Diff 的 display_diff 时 须限制可见行数（硬上限）；超限 须截断并提示 omitted；过大 diff 不得启用 word-level，以免卡死 TUI。

    场景: oversize-diff-truncated-with-omitted
      当 注入含 200 行 display_diff 的 edit 成功结果
      那么 diff 正文截断并提示省略行数
    # verified-by: llmanspec/specs/app-tui-transcript/app-tui-transcript.feature
  @req:r1348
  规则: no-prepend-nav-notices
    产品 TUI 在 travel/fork/resume/切换等重建或瞬时导航通知写入 live scrollback 时，须将此类 ScrollNotice/Error 滚动提示追加到 entries 末尾（跟底时位于输入框上方可滚区域）；不得为提高可见性而把瞬时导航通知 prepend 到 entries 前缀。会话路径上的时间线内容（含 BranchSummary 等按祖先投影的条目）不在本条「瞬时通知」范围。

    场景: travel-notice-trailing-append-headless
      当 重建到叶节点并按产品路径追加路径通告
      那么 通告条目位于 entries 末尾且重建内容次序保持原样
  @req:r1349
  规则: fold-glyph-chevron
    产品折叠标记 Unicode 字形 须为收起可展开 ▸、展开可收起 ▾；Ascii 回退 须为 > / v；标记可视宽 须为 1 列；须尊重 XYLITOL_TUI_GLYPH_SET=ascii。本要求不规定 Bash/Compaction 等无三角块。

    场景: fold-glyph-unicode-and-ascii-fallback
      假如 折叠字形环境未指定（默认 Unicode 集）
      当 读取折叠与展开字形
      那么 折叠为 ▸ 展开为 ▾ 且各占单列
      并且 切换环境变量 XYLITOL_TUI_GLYPH_SET=ascii 并重新读取
      并且 折叠回退为 > 展开回退为 v 且各占单列
  @req:r1350
  规则: per-block-tools-fold-overrides
    产品对 Tool、独立 Diff、Ask 可展开块 须以 tools 族默认展开态加 per-block 覆盖表决定有效展开态（覆盖优先于默认）。左键单击折叠三角列 须仅翻转该块覆盖（或等价单块有效态），不得改变其它块的覆盖。Alt+E 须翻转 tools 族默认展开态并清空该族全部覆盖。本要求不拆 Alt+E 与 compaction 的既有连带（若有）。单块态变 须遵守 ath25：禁止因此对全部历史 Assistant 重做 Markdown 解析。

    场景: tools-per-block-fold-click-and-alt-e-headless
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      当 再左键单击 "t-read" 的工具块三角列
      那么 该块经覆盖表收起且覆盖表只有这一个条目
      当 按下和弦 Alt+E
      那么 工具族默认展开态翻转且块级覆盖清空
  @req:r1351
  规则: thinking-per-id-fold
    产品 Thinking 块 须支持 per-id 展开态（默认 thinking 展开态加 per-id 覆盖，覆盖优先）；每条 Thinking 须有稳定 id，且 live 与 travel/fork/resume 重建路径 须对同一逻辑块使用同一 id。左键单击该块折叠三角列 须仅翻转该 Thinking 的覆盖。Ctrl+T 须翻转 thinking 默认展开态并清空全部 thinking 覆盖。思考通道流式期间外显 须为 Thinking；思考通道结束后 须为 Thought（有可靠思考通道起止墙钟则 Thought {Ns}）。resume / rebuild 须优先用落地的 thinkingElapsedSecs；否则用落盘思考通道起止节点 unix-ms 相减；缺任一端 须省略时长，不得用相邻条目墙钟冒充思考时长，禁止伪造。无 ActivityFold 时流式 thinking 须保持展开（与 att8 一致），结束后可按有效态折叠。

    场景: thinking-per-id-fold-click-and-ctrl-t-headless
      当 以场景构建器回放思考加工具并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      当 再左键单击第一条思考的折叠三角列
      那么 该条思考经覆盖展开且另一条不受影响
      当 按下和弦 Ctrl+T
      那么 思考默认展开态翻转且全部 per-id 覆盖清空
  @req:r1352
  规则: mouse-fold-leaf-header-and-empty-body
    产品在 ApplicationOwned 下，对 att20/att21 范围内叶块（Tool / Diff / Ask / Thinking）：收起时折叠命中 须覆盖块头全宽（含折叠标记）；展开后空选左键点 painted 正文 须亦 toggle 该块。transcript 拖选进行中或已有选区 须忽略折叠命中（不因划过而 toggle，不得因此复制）。夹心助手正文 不得登记为该叶块折叠命中。本要求不涵盖信封/簇头行（见 att31）；Compaction 保持三角列-only（见 att29）。Ask 等待时 Asking questions 整行可点见 att31 与 att33。

    场景: mouse-fold-leaf-triangle-still-toggles-headless
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      当 再左键单击 "t-edit" 的工具块三角列
      那么 该块经覆盖表收起且覆盖表只有这一个条目

    场景: mouse-fold-leaf-header-full-width-headless
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      当 点击 "t-edit" 工具块头行非折叠标记列
      那么 该块经覆盖表收起且覆盖表只有这一个条目

    场景: mouse-fold-leaf-empty-body-collapses-headless
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      当 空选左键单击已展开 "t-read" 的 painted 正文
      那么 该块经覆盖表收起且覆盖表只有这一个条目
  @req:r1353
  规则: activity-nested-envelope-cluster
    产品 须将 live scrollback 中间活动按嵌套 Activity 管理：信封（一轮）套簇套块。信封折叠可见 须为 User + 一行 Worked for + 该轮最后一段助手正文；中间助手正文与 Compaction、工具与 thinking 一并收纳。待办栏（checklist 投影）不得进信封。ScrollNotice 与 Error 不得进信封。信封展开 须仍画出夹在簇之间的中间助手正文，且 须留下 Worked for 头行与展开标记以便再折。簇展开后块级折叠 须仍服从 att20/att21。每信封与每簇 须有稳定 id，且 live 与 travel/fork/resume 重建 须同构。展开任一级 须留下该级头行与展开标记（att19），以便再折；例外：一簇的中间活动全部为 Compaction 时 不得再画簇摘要头，信封展开 须直接露出既有 Compaction 块。信封头、簇头与块头 须与未折叠细账同一列展示；不得用前导缩进表达嵌套层级。

    场景: envelope-fold-hides-inner-keeps-heads-headless
      当 以场景构建器回放四轮活动并按回合结束收纳
      那么 最旧信封折叠为用户行加 Worked for 加末段正文且无内层块
      当 左键单击折叠命中表中的信封三角列
      那么 该信封定点展开且中间助手正文与簇头行重新可见
  @req:r1354
  规则: cluster-summary-envelope-worked-for
    簇摘要行 须只描述真实发生的活动，英文 Title Case。文件层 须互斥：有改写（edit/write 等）则只写 Edited；否则有读或搜索则写 Explored；不得同一头并列 Edited 与 explored。有 shell 才追加 Ran N commands；仅 shell、无文件活动时整行 须为 Ran。N 须为去重 path 计数：恰一个写 `1 file`、多个写 `N files`（c2540：不得以 basename / 路径片段替代计数——具体文件由展开簇内子块呈现；`Explored .` 类输出随计数式天然消失）。无 path 的搜索 不得加成假文件数，但仍可使该簇进入 Explored。混合簇头 不得写入 thinking、MCP 或 compaction。当文件层与 Ran 皆空：仅 thinking 流式未结束 须为 Thinking（无时长）；思考通道结束后 须为 Thought，有可靠思考通道起止墙钟才附 Thought {Ns}；resume 须优先用落地的 thinkingElapsedSecs 恢复 Thought {Ns}；否则用落盘思考通道起止节点 unix-ms 相减；缺任一端 须省略时长，不得用相邻条目墙钟冒充思考时长，禁止伪造；不得流式滴答时长。同一轮中途 thinking 不切簇（att34），与工具或 Ask 同簇时簇头 须走 Edited / Explored / Ran / Used / Asking questions 等真实活动，不得用 Thought 当聚合头；todo_* 与其它未知/MCP 工具一样走 Used，不单开类目。Used 的 N 须为调用次数（与 Ran 同构），同一工具多次调用 须按次累加；仅一次时 须写 Used {短名}；多次 须写 Used N tools。checklist 投影行 不得计入 Used N。仅 MCP 或未知工具 须为 Used；仅 Ask 须为 Asking questions；仅 Compaction 不得画簇头（见 att23）。不得用文件占位虚构 Explored。打开簇进行中 须用 Editing / Exploring / Running（与文件互斥及 Ran 对齐）；封口后 须用 Edited / Explored / Ran。类目计数 须在工具开始时更新；+/- 须仅在有可靠 diff 统计时于工具结束附加，否则省略；不得伪造 +/-。无中间操作 不得生成空簇。信封 Worked for 须以可靠双端墙钟书写时长；缺任一端 须仍可画 Worked for 但省略时长数字；不得伪造时长。

    场景: cluster-head-wording-exclusivity
      当 以场景构建器回放读后改写序列（read old.rs 然后 edit a.rs）
      那么 只读前簇封口为 Explored 1 file 且改写簇头保持 Editing 1 file
      并且 改写结束后无 Edited 错时态
      并且 全帧不出现 Worked for 与 Planning next moves
  @req:r1355
  规则: activity-fold-layered-with-l1
    信封或簇折叠时：被收纳的内层块 不得进入渲染；Alt+E、Ctrl+T、Ctrl+O 与 per-id 块级覆盖 不得改变该折叠外观；被收纳块的折叠三角 不得登记。信封与簇均展开时：既有块级默认态、覆盖表与三角命中 须照常生效。折叠态切换 须遵守 ath25：禁止因此对全部历史助手正文重做 Markdown 解析。

    场景: activity-fold-layered-collapse-hides-blocks-headless
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      那么 簇即为折叠收纳态且无内层块登记
      当 依序按下和弦 Alt+E、Ctrl+T、Ctrl+O
      那么 渲染帧与按键前逐字一致
      当 左键单击折叠命中表中的簇头三角列
      那么 仅该簇被定点展开且内层块行可见
  @req:r1356
  规则: activity-fold-auto-collapse
    产品 须提供 ActivityFold 自动收纳：enabled 默认 true（关闭则全细账，块级折叠仍可用）；keep_recent_turns 默认 2（仅约束 auto_on_turn_end：最近 K 个已结束 Activity 轮信封默认展开，更旧信封默认折叠）；auto_on_rebuild 默认 true（travel/resume/fork 重建后对全部已结束 Activity 轮套折叠信封，不按 keep_recent_turns 留近窗）；auto_on_turn_end 默认 true（回合结束后对超窗已结束轮套折叠）。流式当前轮 不得被收成 Worked for 信封，须走 att33 live window。YAML 键与非法值见 runtime-config。近窗口内从未进入 ActivityFold 的细账 不得被当作 collapseNearest 目标。旧键 auto_l3_distant 不得再作为产品配置面。

    场景: activity-auto-collapse-windows-headless
      当 以场景构建器回放四轮活动并按回合结束收纳
      那么 仅最旧两轮折叠为 Worked for 且近窗轮正文与簇头保持
      当 以重建路径应用重建收纳
      那么 全部四轮折叠为 Worked for
      当 关闭 ActivityFold 重挂并按回合结束收纳
      那么 全帧无 Worked for
  @req:r1357
  规则: activity-fold-markers-and-hints
    信封与簇头行折叠标记 须遵守 att19（Unicode ▸/▾；Ascii >/v）。旁注 须为当前绑定的完整和弦括号；默认展开旁注 须对应 Alt+Shift+E，默认收纳旁注 须对应 Ctrl+Alt+Shift+E；改绑后旁注 须跟随当前绑定。信封/簇头 不得写 (Alt+E)，以免与块级展开混淆。Ask 等待的 Asking questions 不得画折叠三角。不得画 Planning next moves。

    场景: envelope-cluster-markers-and-chords-headless
      当 以场景构建器回放四轮活动并按回合结束收纳
      那么 折叠信封与收起簇头旁注均为 Alt+Shift+E
      当 左键单击折叠命中表中的簇头三角列
      那么 展开簇头旁注为 Ctrl+Alt+Shift+E 且信封簇头行不带 (Alt+E)
      并且 全帧无 Planning next moves
  @req:r1358
  规则: activity-expand-collapse-nearest
    产品 须提供跨面动作语义 activity.expandNearest 与 activity.collapseNearest（TUI 键位 id 为 app.activity.expandNearest / app.activity.collapseNearest；默认和弦 Alt+Shift+E / Ctrl+Alt+Shift+E，可改绑）。expandNearest 须将距输入最近的折叠信封先展开；若该信封已展开则展开最近折叠簇。collapseNearest 须将距输入最近的展开簇先折；若无则折最近展开信封。无合格目标时两动作 须静默（无报错、无 toast、无态变）。启发式 须按 entries 序靠近输入；不得抢 Editor 常驻焦点。一对动作 须覆盖嵌套一级步进；不得另开信封/簇专用默认键；弱终端 不得另发官方第二默认和弦。

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
  @req:r1359
  规则: compaction-fold-triangle-mouse
    产品在 ApplicationOwned 下，Compaction 可折块 须提供折叠三角列（字形遵守 att19）。左键单击该三角列 须仅翻转全局 compaction 展开态（compaction_expanded 或等价）；不得清空 tools 族覆盖，不得翻转 tools 族默认展开态。单击摘要正文、旁注和弦或其它非三角列 不得因此 toggle。键盘 Alt+E 与 compaction 的既有连带（若有）不在本条拆改范围。态变 须遵守 ath25：禁止因此对全部历史 Assistant 重做 Markdown 解析。

    场景: compaction-fold-triangle-mouse-headless
      当 以场景构建器回放压缩事件并封轮挂载交互面
      当 左键单击 Compaction 块的折叠三角列
      那么 Compaction 全局展开态翻转
      并且 工具族覆盖表仍为空且工具族默认态未变

    场景: compaction-fold-summary-does-not-toggle-headless
      当 以场景构建器回放压缩事件并封轮挂载交互面
      当 点击 Compaction 块摘要正文列而非折叠标记列
      那么 Compaction 全局展开态未变
      并且 工具族覆盖表仍为空且工具族默认态未变
  @req:r1361
  规则: output-viewport-hint-mouse
    产品在 ApplicationOwned 下，对渲染 Ctrl+O 输出视口 expand/collapse hint 的块（含 Tool、Bash、独立 Diff 等凡展示该 hint 者）：左键单击可见 expand hint 带 须仅翻转该块的输出视口展开态（全局默认 + 按块覆盖，或等价），不得波及其他块；展开态块尾渲染的 fold 提示带单击 须仅把该块折回。键盘 Ctrl+O（app.tools.expand）须翻转全局默认并清除按块覆盖（与 Alt+E / Ctrl+T 的默认+覆盖模型同构）。命中几何 须为 hint 文案可点带（与叶块 L1 折叠命中分层，后登记赢）。Bash 不得另补块级 L1 折叠三角；其输出高度交互 须走本条 Viewport。硬截断禁展开（att16）时 不得因点击 hint 而展开全文。态变 须遵守 ath25（单块覆盖切换仅重绘该块）。

    场景: output-viewport-hint-band-mouse-headless
      当 以场景构建器回放多行输出的工具并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      当 点击输出的 Ctrl+O 提示带
      那么 提示带点击仅翻转该块输出视口且 Ctrl+O 翻全局并清按块覆盖

    场景: output-viewport-fold-per-block-headless
      当 以场景构建器回放两个多行输出工具并封轮挂载交互面
      当 点击第一个工具的 Ctrl+O 提示带
      那么 仅该块展开且另一块保持折叠且折叠带可点回折
  @req:r1362
  规则: envelope-cluster-fold-marker-mouse
    产品在 ApplicationOwned 下，对信封与簇头行：左键单击块头行全宽（含折叠标记，字形遵守 att19/att27）须只 toggle 该级（信封或该簇），不得使用 expandNearest/collapseNearest 启发式代替定点。展开后的嵌套块与夹心助手正文 不得登记为该级命中。对 Asking questions：整行可点 须展开打开簇。分层继承 att25：信封或簇折叠时 Alt+E、Ctrl+T、Ctrl+O 与 per-id 块级覆盖 不得改变该折叠外观；被收纳块级折叠三角 不得登记。态变 须遵守 ath25。

    场景: cluster-fold-marker-mouse-precise-headless
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      那么 仅该簇被定点展开且内层块行可见

    场景: cluster-header-full-width-mouse-headless
      当 以场景构建器回放读后改写序列并封轮挂载交互面
      当 点击簇头行的摘要正文列
      那么 仅该簇被定点展开且内层块行可见
  @req:r1363
  规则: remaining-fold-targets-unified-hit
    产品对 att29–att31 剩余可点折叠目标（Compaction、OutputViewport、信封、簇）须登记进与 att20/att21 同一折叠命中表及产品指针命中管线；不得另开第二套命中管道。transcript 拖选进行中 须忽略这些折叠命中（与 att22/ath33 同构）。本条不要求新增 host latch 管道（ath33 已覆盖空选单击与拖选忽略）。

    场景: remaining-fold-targets-unified-hit-table-headless
      当 以场景构建器回放压缩加多行输出并封轮挂载交互面
      当 左键单击折叠命中表中的簇头三角列
      那么 Compaction、输出提示带与块级三角登记于同一命中表且定点可点
  @req:r1364
  规则: activity-live-window
    流式当前轮 须使用 live window，不得整轮收成 Worked for 信封。不得画 Planning next moves 占位行；尚无可展示助手正文且无 thinking/Ask/工具时 不得画空助手气泡，busy 短词走状态条 Working 或 Running {name}。Thinking 流 须画 Thinking 簇头（可点展开，默认折叠正文，无时长）；思考通道结束后 须更新为 Thought，有可靠思考通道起止墙钟 须写 Thought {Ns}（如 Thought 17s）；起止 须不含随后正文或工具流；已封 Thought 簇 不得因后续 Thinking 流改回 Thinking；resume 优先落地 thinkingElapsedSecs，否则思考通道节点 unix-ms 相减，缺戳则省略时长；不得用相邻条目墙钟冒充思考时长；不得流式每帧刷新时长。仅 thinking 的簇 不得同时画 Thinking/Thought 簇头与 thinking L1 头（合并为一行）。与工具同簇时簇头走 att24 真实活动，L1 仍可 Thought {Ns}。Ask 等待 须显示 Asking questions，且 Ask 块 须可交互、不得折没。打开簇有进行中或已结束的工具时 须画带折叠三角的簇头（att24 进行时 Editing / Exploring / Running）；流式工具块 须作为该簇子项，默认折叠，点簇头三角 须可展开（含流式）。同一打开簇内工具结束或多次调用归并 不得自动收起已展开的子项。paint 不得每帧改折叠态。不得把进行中工具只画成无三角的 live 尾行而藏起块。不得把无改写的读文件写成 Editing；不得虚构 Explored。助手正文 不得被收进簇。已封簇与更早内容 不得因打开簇更新而重算。点 Asking questions 须揭开已算好的 sealed 细账。状态条短词 须仍为 Working 或 Running {name}。已结束轮何时套 Worked for 信封见 att26，不在 live window 内提前套。

    场景: live-window-unenveloped-headless
      当 以场景构建器渲染流式思考中的 live window
      那么 出现 Thinking 簇头且无 Thought 与 Ctrl+T 旁注
      当 以场景构建器渲染含助手正文的 live window
      那么 助手正文可见且仍无信封封套
      并且 全帧不出现 Worked for 与 Planning next moves
  @req:r1365
  规则: cluster-split-assistant-body
    簇边界 须以已可展示的助手正文划分：正文第一个非空白字符 须封口上一打开簇。Thinking、工具、Ask、Diff 不得单独切簇；Compaction 例外：须封口当前打开簇并自成单例簇独立成块（簇头呈现遵守 att23），其后中间活动 须另起新簇；连续多条 Compaction 各自成块。同一轮中途再思考仍留在打开簇内，簇头措辞见 att24。同一条目序下 live 与 resume/rebuild 切分 须同构。待办栏不在对话条目中，不得参与切簇。

    场景: compaction-seals-cluster-headless
      当 以场景构建器回放工具后压缩再工具序列
      那么 压缩块独立成块且前后工具簇各自成簇
      并且 全帧不出现 Worked for 与 Compaction 簇摘要头

    场景: assistant-body-seals-cluster-headless
      当 以场景构建器在两个工具活动之间插入助手正文
      那么 正文封口前簇且新簇在其下方独立开口
  @req:r1366
  规则: explore-cluster-head-category-counts
    含探索活动（读 / 搜索调用）的簇，其簇头 须在 att24 文件层词形之后附加类目计数后缀，形如 `· 5 reads · 2 searches`：计数为调用次数（非去重路径数，与 att24 文件计数可区分），仅列非零类目，无路径的搜索调用计入 searches。进行时 须用 Exploring 词形且后缀计数随工具开始更新（对齐 att24 计数时机）。簇内子块的既有默认收纳与展开语义（点击簇头 / att28 就近展开）须保持不变；本条不改变 att23 嵌套结构、att24 基础词形、att26 回合窗语义。

    场景: explore-head-suffix-counts
      当 以场景构建器回放多读多检索序列（三读两检索且检索无路径）
      那么 封口簇头 同时含文件计数与类目计数后缀（3 files · 3 reads · 2 searches）
      当 以场景构建器回放同类别单序列（三读）
      那么 后缀 只列该非零类目

    场景: explore-head-suffix-live-update
      假如 探索簇流式进行中
      当 新的读段或检索段开始
      那么 簇头后缀计数 随工具开始更新且进行时词形 为 Exploring
  @req:r1367
  规则: todo-tool-block-body-checklist
    todo_* 工具块展开 body 须渲染清单形态：每条目一行（状态字形 + content），状态字形与待办栏共用同一张表；不得把调用 args 或结果的原始 items JSON 当作块 body。is_error 结果 须保持错误文本原样可见，不得套用清单渲染。todo_* 成功结果解析为空列表时，展开 body 须为一行显式空态提示（固定英文词 `(empty list)`），不得留空 body 静默。历史重建与直播 须共用同一渲染 helper，重建形态 须与直播幂等（att50 幂等语义延伸到块 body）。

    场景: todo-block-body-renders-checklist
      当 以场景构建器回放 todo_rewrite 成功调用的直播与 travel 重建
      那么 两种路径的块 body 均为清单行形态且逐行一致，不出现原始 items JSON

    场景: todo-block-empty-list-body-hint
      当 以场景构建器回放 todo_rewrite 空列表结果的直播与 travel 重建
      那么 两种路径的块 body 均为空态提示行 `(empty list)` 且逐行一致，不留空 body
  @req:r2893
  规则: product-ao-scrollbar-gutter
    产品 ApplicationOwned 会话必须默认开启最右列滚动槽（包层 opt-in 契约的产品侧翻转）：transcript 最右保留 1 列，跳转顶部/跳转底部可点击并作用于应用视口；产品宿主必须把滚动槽命中区随帧并入指针命中表且点击分派先于折叠命中。Inline 会话不得接线（仍走终端 scrollback）。

    场景: product-gutter-click-jumps
      假如 产品 ApplicationOwned 会话已向下滚动
      当 产品会话左键单击跳转顶部按钮字符
      那么 产品应用视口滚动位置归零且无选区
