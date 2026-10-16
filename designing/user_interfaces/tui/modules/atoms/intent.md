# atoms

截断 `…`、spinner、选中反转（bg on-surface、fg surface，子孙 inherit）。

- glyph 档 `unicode`（`❯` / `▸▾`）与 `ascii`（`>` / `>v`）由配置切换，禁止运行时字体探测。
- busy spinner 用 braille 帧，**不算** glyph 档。
- 用户消息内联 `$skill` 用 skill-ref token，不是 accent。工具行禁止 `⚙`。
