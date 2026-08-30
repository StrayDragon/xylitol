# language: zh-CN
# capability: package-tui-markdown
# purpose: token 友好的 Markdown 终端渲染：SGR 层级、明文链接、无围栏代码块、空格对齐表。
# scope: packages/xylitol-tui/, docs/architecture/

功能: package-tui-markdown

  @req:ptm1 @human
  场景: heading-sgr-no-hash
    - Markdown 渲染标题时 MUST 用主题色与 bold/underline 表达 H1–H6 层级；可见文本 MUST NOT 包含 # 或 ## 等井号前缀。

  @req:ptm2 @human
  场景: link-text-url
    - Markdown 渲染链接时 MUST 输出可见形态 text (url)；图片 MUST 输出 alt (url)（无 alt 则用 url）；MUST NOT 仅依赖不可选中的 OSC 或丢弃 URL。

  @req:ptm3 @human
  场景: code-block-no-fence
    - Markdown 代码块 MUST 仅输出代码内容行（可选缩进与 highlight_code 高亮）；MUST NOT 输出围栏行、语言标签条或行号墙。

  @req:ptm4 @human
  场景: inline-emphasis-display
    - Markdown 粗体与斜体 MUST 仅用 SGR（theme.bold / theme.italic）表达，可见文本 MUST NOT 保留 ** 或 * 星号包裹；行内代码与删除线的可见文本 MUST 分别保留 ` 与 ~~ 标记。

  @req:ptm5 @human
  场景: quote-no-box
    - Markdown 引用 MUST 仅用 quote 色与 italic（或等价 SGR）；MUST NOT 使用竖线或盒线装饰前缀。

  @req:ptm6 @human
  场景: table-space-align
    - Markdown 表格 MUST 以空格对齐列宽显示；表头 MAY bold 与 underline；MUST NOT 使用盒线表字符；MUST NOT 为装饰输出管道符。

  @req:ptm7 @human
  场景: list-and-hr
    - Markdown 列表 MUST 使用 - 或数字点号标记且嵌套用空格缩进；MUST NOT 使用树线装饰。分隔线 MUST 为短线或空行；MUST NOT 拉满近全宽装饰线墙。任务列表 MUST 保持 - [ ] / - [x] 与正文同一行。

  @req:ptm8 @human
  场景: highlight-callback-only
    - 语法高亮 MUST 仅经 MarkdownTheme.highlight_code 可选回调注入；xylitol-tui 默认依赖 MUST NOT 捆绑 syntect。fg 着色在内联阶段完成；若设置 bg_color 则 MUST 在行宽 padding 后经 apply_background_to_line 铺满。
