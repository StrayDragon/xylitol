# language: zh-CN
# capability: package-tui-editor
# purpose: xylitol-tui Editor：视觉行图、粘滞列、PasteBurst、历史导航与边框色 API。
# scope: src/, tests/

功能: package-tui-editor

  @req:r1598
  规则: visual-line-map
    Editor MUST 经 build_visual_line_map(width) 从逻辑行构建 visual-line map，将每个 wrapped visual segment 映射到 logical line index、start column 与 byte length。map MUST 考虑宽于 width 的 word-wrap，空行 MUST 占一行 visual line。map 是跨 wrapped 行垂直光标移动的基础。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1599
  规则: sticky-column-vertical-move
    Editor MUST 经 compute_vertical_move_column(current_col, src_max, tgt_max) 实现粘滞列垂直光标移动，遵循 P/S/T/U 决策表：（P=has preferred，S=cursor in middle，T=target shorter than current，U=target shorter than preferred）。editor MUST 存储 preferred_visual_col 与 snapped_from_cursor_col。set_cursor_col MUST 清除两者。move_cursor(dl, dc) 在 dl != 0 时 MUST 委托 move_to_visual_line。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1600
  规则: page-scroll
    Editor MUST 按宽度相关视觉行布局实现 pageScroll（或等价滚动入口），page size 为 max(5, terminal_rows * 30 / 100)。光标 MUST 按 page 大小移动 visual lines，而非硬编码 logical-line 偏移。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1601
  规则: paste-burst-integration
    Editor MUST 集成 PasteBurst（c415）。每次 plain character 插入时 MUST 调用 on_plain_char(now)。Enter（submit 或 newline）时 MUST 检查 should_insert_newline_instead_of_submit(now)；若为 true MUST 插入 newline 而非 submit。非 printable key 或显式 paste 时 MUST 调用 reset()。now Instant MUST 作为参数传入（非 wall-clock），遵循 c415 injectable-clock 模式。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1602
  规则: history-navigation
    Editor MUST 提供 navigate_history(direction: isize)，使用 setTextInternal 与 cursorPlacement（上箭头 'start'，下箭头 'end'）。exitHistoryBrowsing() MUST 在每个应退出 history 模式的编辑动作（insert、delete、yank、submit）入口调用。从 draft 状态进入 history 时，当前 editor 状态 MUST 保存为 history_draft。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1603
  规则: highlight-via-callback
    Markdown 代码块高亮 MUST 经 MarkdownTheme.highlight_code 回调注入；xylitol-tui 默认依赖 MUST NOT 包含 syntect；真实高亮实现 MUST 位于 optional feature（如 highlight）或应用面适配层。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1604
  规则: highlight-safety-limits
    注入的 syntect（或等价）高亮 MUST 对过大输入施加字节与行数上限；超限时 MUST 回退为未高亮纯文本行，MUST NOT 阻塞事件循环。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1605
  规则: set-border-color
    Editor MUST 提供 set_border_color（或等价 API）以替换边框 ANSI 包装闭包，供 host 在 bash 模式等场景切换操作区边框色；MUST NOT 要求重建整个 Editor 实例。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1606
  规则: paste-collapse-expand
    Editor MUST 在 bracketed paste 内容超过 10 行或超过 1000 字符时，将缓冲区内文本替换为 [paste #N +L lines] 或 [paste #N C chars] 占位，并在内部 pastes 表保留原文；get_text MUST 返回含占位符的显示文本；get_expanded_text MUST 将每个有效 id 的完整 marker（含 +lines/chars 后缀）替换为原文，MUST NOT 残留 +N lines] 或 N chars] 碎片。短粘贴 MUST 直接插入原文。
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
  @req:r1607
  规则: thinking-border-apply
    Editor MUST 能在不重建实例的前提下经 set_border_color（ed08）或薄 helper apply_thinking_border 应用 Palette 给出的 thinking 边框闭包；应用后 render 的边框行 MUST 经该闭包着色；MUST NOT 在 Editor 内硬编码产品 domain ThinkingLevel。
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # verified-by: packages/xylitol-tui/src/components/editor/tests.rs
