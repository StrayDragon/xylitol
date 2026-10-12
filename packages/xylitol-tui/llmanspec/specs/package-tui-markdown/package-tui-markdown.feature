# language: zh-CN
# capability: package-tui-markdown
# purpose: token 友好的 Markdown 终端渲染：SGR 层级、明文链接、无围栏代码块、空格对齐表。
# scope: src/components/markdown.rs, src/components/mermaid_ascii.rs, src/theme/palette.rs, src/lib.rs
# reviewed-through: c2890

功能: package-tui-markdown

  @req:r1643
  规则: heading-sgr-no-hash
    Markdown 渲染标题时 须 用主题色与 bold/underline 表达 H1–H6 层级；可见文本 不得 包含 # 或 ## 等井号前缀。
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
  @req:r1644
  规则: link-text-url
    Markdown 渲染链接时 须 输出可见形态 text (url)；图片 须 输出 alt (url)（无 alt 则用 url）；不得 仅依赖不可选中的 OSC 或丢弃 URL。
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
  @req:r1645
  规则: code-block-no-fence
    Markdown 代码块 须 仅输出代码内容行（可选缩进与 highlight_code 高亮）；不得 输出围栏行、语言标签条或行号墙。
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
  @req:r1646
  规则: inline-emphasis-display
    Markdown 粗体与斜体 须 仅用 SGR（theme.bold / theme.italic）表达，可见文本 不得 保留 ** 或 * 星号包裹；行内代码与删除线的可见文本 须 分别保留 ` 与 ~~ 标记。
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
  @req:r1647
  规则: quote-no-box
    Markdown 引用 须 仅用 quote 色与 italic（或等价 SGR）；不得 使用竖线或盒线装饰前缀。
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
  @req:r1648
  规则: table-space-align
    Markdown 表格 须 以空格对齐列宽显示；表头 MAY bold 与 underline；不得 使用盒线表字符；不得 为装饰输出管道符。
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
  @req:r1649
  规则: list-and-hr
    Markdown 列表 须 使用 - 或数字点号标记且嵌套用空格缩进；不得 使用树线装饰。分隔线 须 为短线或空行；不得 拉满近全宽装饰线墙。任务列表 须 保持 - [ ] / - [x] 与正文同一行。
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
  @req:r1650
  规则: highlight-callback-only
    语法高亮 须 仅经 MarkdownTheme.highlight_code 可选回调注入；xylitol-tui 默认依赖 不得 捆绑 syntect。fg 着色在内联阶段完成；若设置 bg_color 则 须 在行宽 padding 后经 apply_background_to_line 铺满。

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）
    # re-review(c2890): 复审结论——既有标题/链接/围栏铬/高亮回调规则不变；本票只新增闭合 mermaid 盒线与失败回源（r1947–r1949）。（2026-10-11）
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
  @req:r1936
  规则: streaming-partial-closing-fence
    Markdown 在流式或完整文本以未闭合代码围栏结尾、且最后一行是短于该围栏 opener 的同字符反引号或波浪线时，须 在解析前去掉该半截闭合行。补全闭合围栏后，代码块可见行数 须 与去掉半截时相同。已闭合围栏、围栏外正文、以及并非 closer 前缀的末行 不得 被误删。不得 因此画出围栏铬或语言标签条（仍遵守代码块无围栏行）。不得 把修剪做成产品私有第二套解析。
    # verified-by: packages/xylitol-tui/src/components/markdown.rs
    场景: streaming-partial-fence-height-stable
      当 渲染以半截闭合围栏结尾的围栏代码再补全该闭合围栏
      那么 两次代码块可见行数相同且无围栏铬
  @req:r1947
  规则: mermaid-fence-ascii-or-source
    Markdown 渲染围栏语言首词为 mermaid（大小写不敏感）的已闭合代码块时：能画成盒线且自然列宽不超过可用宽，须 输出 Unicode 盒线行，且 不得 输出围栏行、语言标签条或该围栏源码；画不成时（画图返回错误）须 先输出错误色提示「无法渲染 mermaid」，再按普通代码块呈现该代码块内源码（即使非法也原样；源码比提示多一档缩进，行内代码色，保持源码换行不按散文折）。失败呈现 不得 套标题装饰框，不得 输出围栏行。

    场景: mermaid-closed-flowchart-draws-boxes
      当 渲染已闭合的最小 flowchart TD mermaid 围栏
      那么 可见行含盒线且无围栏铬

    场景: mermaid-unknown-type-stays-source
      当 渲染已闭合但画不成盒线的 mermaid 围栏
      那么 可见行是代码块内源码并带无法渲染提示且无盒线图
  @req:r1948
  规则: mermaid-mode-final-open-fence
    Markdown mermaid 围栏默认模式为 final：当源在文件末尾仍有未闭合的 mermaid 围栏时，须 按普通代码块呈现，不得 输出盒线图，不得 输出无法渲染提示。模式为 off 时任何 mermaid 围栏 须 按普通代码块呈现，不得 输出无法渲染提示。模式为 streaming 时未闭合围栏 MAY 尝试画图，失败仍回源并带无法渲染提示。

    场景: mermaid-final-open-fence-stays-source
      当 以默认 final 渲染末尾未闭合的 mermaid 围栏
      那么 可见行是源码且无盒线图

    场景: mermaid-off-never-draws
      当 以 off 模式渲染已闭合的最小 flowchart TD mermaid 围栏
      那么 可见行是源码且无盒线图

    场景: mermaid-streaming-open-garbage-falls-back
      当 以 streaming 模式渲染末尾未闭合且画不成盒线的 mermaid 围栏
      那么 可见行是代码块内源码并带无法渲染提示且无盒线图
  @req:r1949
  规则: mermaid-too-wide-fallback
    已闭合 mermaid 围栏若盒线自然列宽超过本次渲染可用宽（已扣除该代码块既有缩进），须 整块回退为代码块内源码并带无法渲染提示，不得 把盒线折行，不得 输出围栏行。
    场景: mermaid-too-wide-falls-back
      当 在窄于盒线自然宽的宽度渲染已闭合 mermaid 围栏
      那么 可见行是代码块内源码并带无法渲染提示且盒线未折行
