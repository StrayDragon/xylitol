# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-editor

  @req:ed01
  场景: build-vl-map-wraps-long-lines
    假如 30 列 editor 中有一行 100 字符
    当 build_visual_line_map(30) 被调用
    那么 map 含 4 个 visual line 条目，logical_line、start_col、len 正确

  @req:ed02
  场景: sticky-col-up-arrow
    假如 光标在第 2 行第 15 列，该行跨 3 个 visual lines
    当 按下 cursor up
    那么 光标移到上一 logical line 对应 visual line 的第 15 列（或较短行尾）

  @req:ed02
  场景: sticky-col-cleared-on-horizontal
    假如 从第 15 列 cursor up 到第 8 列（行更短）设置 preferred_visual_col 为 15
    当 word_left 被按下
    那么 preferred_visual_col 设为 None

  @req:ed03
  场景: page-scroll-moves-by-pagesize
    假如 terminal rows=40，page size=12
    当 page_down 被按下
    那么 光标经 move_to_visual_line 下移 12 个 visual lines，而非 5 个 logical lines

  @req:ed04
  场景: paste-burst-enter-suppressed
    假如 9 个 fast chars 各 1ms 内到达，触发 paste burst，再 Enter
    当 handle_input 收到 Enter
    那么 PasteBurst 对 should_insert_newline_instead_of_submit 返回 true，插入 newline 而非 submit

  @req:ed04
  场景: paste-burst-reset-on-nonprintable
    假如 burst 进行中
    当 按下 CursorLeft（non-printable）
    那么 paste_burst.reset() 被调用，清除 burst 状态

  @req:ed05
  场景: history-draft-restored
    假如 用户 up-arrow 浏览 2 条 history
    当 down-arrow 越过最后一条回到 history_index=-1
    那么 保存的 history_draft 以原 editor 内容恢复

  @req:ed06
  场景: demo-injects-syntect
    假如 feature highlight 启用
    当 agent_demo 渲染含 rust fence 的助手消息
    那么 输出行含 ANSI 颜色序列

  @req:ed06
  场景: default-no-syntect-dep
    当 cargo metadata 默认 features
    那么 xylitol-tui 默认依赖树不含 syntect

  @req:ed07
  场景: oversize-fallback
    假如 代码超过字节上限
    当 调用 highlight_code
    那么 返回未高亮纯文本行且不 panic

  @req:ed08
  场景: border-swap
    假如 Editor 已构造
    当 调用 set_border_color 后 render
    那么 边框行经新闭包着色

  @req:ed09
  场景: long-paste-collapses
    假如 Editor 收到超过 10 行的 bracketed paste
    当 粘贴完成
    那么 get_text 匹配 [paste #N +L lines] 且 pastes 表含原文

  @req:ed09
  场景: expanded-equals-original
    假如 长粘贴已折叠为 marker
    当 调用 get_expanded_text
    那么 返回值等于粘贴原文且不含 [paste #

  @req:ed09
  场景: short-paste-inline
    假如 粘贴不超过 10 行且不超过 1000 字符
    当 粘贴完成
    那么 get_text 含原文且无 [paste # 占位

  @req:ed10
  场景: apply-high-border
    假如 Editor 已构造且 Palette 为 Dark
    当 apply_thinking_border(High) 或等价 set_border_color 后 render
    那么 边框行含 High 对应真彩序列

  @req:ed10
  场景: swap-without-rebuild
    假如 同一 Editor 实例
    当 先 Medium 再 High 应用边框
    那么 两次 render 边框色不同且实例未重建
