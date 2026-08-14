# trust-prompt

项目 Trust 闸：ChoicePrompt **替换 editor 槽**。禁止 stdio 数字菜单。

- Trust / Trust parent → 写入后再进产品 TUI。Do not trust → 写入 deny 后退出。
- Esc / Ctrl+C 取消：不写 store，恢复终端后退出（不进 TUI）。
- 不等于内置 `ask` 工具。prompt 含换行按行渲染，禁止粘成一行。
