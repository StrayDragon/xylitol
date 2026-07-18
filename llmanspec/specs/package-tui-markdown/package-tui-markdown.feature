# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-markdown

  @req:ptm1
  场景: h1-no-hash
    当 渲染一级标题 Hello
    那么 剥 ANSI 后行内含 Hello 且不含井号前缀

  @req:ptm1
  场景: h2-underline-path
    当 渲染二级标题
    那么 输出经 heading 主题且无井号前缀

  @req:ptm2
  场景: link-form
    当 渲染 [docs](https://ex.com)
    那么 剥 ANSI 后含 docs (https://ex.com)

  @req:ptm2
  场景: image-form
    当 渲染图片含 alt 与 url
    那么 剥 ANSI 后含 alt (url) 形态

  @req:ptm3
  场景: no-fence
    当 渲染 fenced rust 代码块
    那么 剥 ANSI 后不含三反引号围栏行且含代码正文

  @req:ptm4
  场景: inline-code-ticks
    当 渲染行内 code
    那么 剥 ANSI 后含反引号包裹的 code

  @req:ptm4
  场景: bold-sgr-no-stars
    当 渲染粗体词
    那么 剥 ANSI 后不含 ** 且含词本身；原始输出含 bold SGR

  @req:ptm4
  场景: italic-sgr-no-stars
    当 渲染斜体词
    那么 剥 ANSI 后不含成对 * 包裹且含词本身

  @req:ptm5
  场景: quote-no-bar
    当 渲染引用段落
    那么 剥 ANSI 后不含竖线装饰前缀

  @req:ptm6
  场景: table-spaces
    当 渲染两列表格
    那么 剥 ANSI 后列空格对齐且无盒线字符

  @req:ptm7
  场景: list-markers
    当 渲染无序与有序列表
    那么 输出含 - 与数字点号且无树线

  @req:ptm8
  场景: no-default-syntect
    当 未启用 highlight feature 的默认构建
    那么 Markdown 仍可渲染且包默认依赖无 syntect
