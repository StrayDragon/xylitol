# language: zh-CN
# managed by llman sdd partition-migrate
功能: package-tui-editor-autocomplete

  @req:ea01
  场景: slash-trigger-on-tab
    假如 editor 已注册 SlashCommandSource 且光标在 /hel 之后
    当 按下 Tab
    那么 从 SlashCommandSource 获取建议且 SelectList 出现在 editor 下方

  @req:ea01
  场景: at-path-trigger
    假如 editor 已注册 AtPathSource 且目录含某文件
    当 用户输入 @
    那么 AtPathSource 在 SelectList popup 中列出目录项

  @req:ea02
  场景: selectlist-enter-applies
    假如 autocomplete popup 显示且首项预选中
    当 按下 Enter
    那么 item value 替换光标处前缀且 popup 关闭

  @req:ea02
  场景: selectlist-esc-cancels
    假如 autocomplete popup 显示中
    当 按下 Escape
    那么 popup 关闭且不修改文本

  @req:ea03
  场景: backspace-updates-suggestions
    假如 autocomplete 显示建议中
    当 backspace 从 prefix 删除一字符
    那么 active source 刷新建议以匹配新 prefix

  @req:ea04
  场景: rapid-tabs-drop-stale
    假如 三次 Tab 快速连续到达
    当 仅最后一次的建议应用
    那么 前两次因 start_token 递增被丢弃

  @req:ea00
  场景: shell-present
    当 列出 capability package-tui-editor-autocomplete
    那么 delta 含 ea05–ea06

  @req:ea05
  场景: register-stub
    假如 注册含测试桩的 sources
    当 在桩触发前缀下 Tab
    那么 popup 由该桩提供建议且 Slash/AtPath 仍可独立工作

  @req:ea05
  场景: empty-ok
    假如 仅注册 Slash
    当 输入不触发 /
    那么 无 popup 且无额外源探测副作用

  @req:ea06
  场景: narrow-clamp
    假如 终端宽较窄且建议标签很长
    当 打开 popup 并 render
    那么 每行 visible_width 不超过给定 width
