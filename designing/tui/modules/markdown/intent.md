# markdown

助手正文：终端里直观、安静；框选粘贴省 token。包 Markdown 已按本节收敛。

## 原则

1. Copy = 可见字符。层级用 SGR（色/bold/underline），少装饰盒线。
2. 链接/图片 URL **必须明文**：`text (url)`。
3. 行内代码保留 `` `code` ``；删除线保留 `~~`；粗体/斜体 **无**可见 `**`/`*`。
4. 标题 **MUST NOT** `#` 前缀。H1 accent+bold+underline；H2 accent+bold；H3–4 on-surface+bold；H5–6 muted。
5. 代码块：无围栏、无语言标签条、无行号墙。
6. 表：空格对齐，**MUST NOT** 盒线表。引用 gutter `│ ` 是结构标记。
7. 列表 `- ` / `1. `；任务 `- [ ]` / `- [x]` 与正文同行。

色：正文 assistant；链接 accent；行内代码 success；引用 muted。
