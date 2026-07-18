# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui-chrome

  @req:atc1
  场景: idle-status-zero-rows
    当 空闲 UiRoot apply_ui_model 后 render
    那么 无独立 Working 行且 status 槽占 0 行

  @req:atc1
  场景: busy-status-not-in-footer
    当 busy 且 status=Working 时 render
    那么 存在独立 status 文案且 footer 不含 Working

  @req:atc2
  场景: footer-thinking-label
    假如 当前 thinking level 为 medium
    当 渲染 footer
    那么 末行含 model 与 • medium（或等价 thinking 标签）且仍为单行

  @req:atc3
  场景: ascii-glyph-prefix
    当 配置 glyph=ascii 后渲染用户行
    那么 用户前缀为 ascii 档字符

  @req:atc4
  场景: index-lists-components
    当 打开 DESIGN.md
    那么 组件索引列出 design/ 下核心文档路径

  @req:atc4
  场景: diff-block-exists
    当 实现 Diff 渲染前查阅设计
    那么 design/diff-block.md 存在且含 unified / word-level / CJK MUST

  @req:atc5
  场景: default-dark
    假如 未启用 auto
    当 构造 demo
    那么 theme_mode 为 Dark

  @req:atc5
  场景: auto-osc11-light
    假如 已启用 auto
    当 注入近白 OSC11 背景
    那么 theme_mode 为 Light

  @req:atc5
  场景: auto-off-ignores-env
    假如 未启用 auto 且 COLORFGBG 为亮
    当 调用 apply_theme_detect
    那么 theme_mode 仍为 Dark

  @req:atc6
  场景: idle-editor-short
    假如 空闲且 editor 文本为空
    当 render UiRoot
    那么 操作区内容可见行数显著少于满 max_vis 空盒（仍含上下边框与光标行）

  @req:atc6
  场景: draft-grows
    假如 Editor 已有多行草稿
    当 render
    那么 可见行随草稿增加且不超过既有 max_vis 上限

  @req:atc7
  场景: busy-spinner-row
    假如 UiModel busy 且 status=Working
    当 render
    那么 存在独立 status 行含 spinner 帧或 accent 标记且 footer 无 spinner

  @req:atc8
  场景: user-bg-row
    假如 scrollback 含用户消息
    当 render
    那么 用户行带 user-message-bg 染色（或测试可观测的 bg 应用痕迹）

  @req:atc9
  场景: compacting-one-row
    假如 UiModel Busy status=Compacting
    当 UiRoot render
    那么 独立 status 至多 1 行且含 Compacting

  @req:atc9
  场景: retry-one-row
    假如 UiModel Busy status=Retry 1/3
    当 UiRoot render
    那么 独立 status 至多 1 行且含 Retry

  @req:atc10
  场景: abort-clears-status
    假如 Busy 且 status=Working
    当 用户 abort
    那么 status 槽 0 行且 scrollback 含 Aborted

  @req:atc11
  场景: footer-updates
    假如 选定新模型成功
    当 下一帧 footer
    那么 含新模型标签

  @req:atc12
  场景: busy-keeps-leading-blank
    假如 产品 host busy
    当 渲染一帧
    那么 status 区含前导空行再 spinner 行且紧贴 editor

  @req:atc12
  场景: idle-one-blank
    假如 产品 host idle
    当 渲染一帧
    那么 editor 上方恰好一行空白且无 spinner

  @req:atc13
  场景: heuristic-tilde
    假如 estimate provenance 为 Heuristic
    当 渲染 footer token 字段
    那么 文案为 used ~N tokens

  @req:atc13
  场景: unknown-question
    假如 estimate provenance 为 Unknown
    当 渲染 footer token 字段
    那么 文案为 used ? tokens

  @req:atc14
  场景: travel-refreshes-token
    假如 用户 travel 到另一 leaf
    当 下一帧 footer
    那么 token 字段对应该 leaf 的 estimate

  @req:atc14
  场景: no-per-delta-encode
    假如 正在流式生成
    当 仅处理 TextDelta
    那么 不因每个 delta 触发全文 tokenizer.encode

  @req:atc15
  场景: apply-light
    假如 当前为 dark LayoutTheme
    当 apply 主题名 light
    那么 UiRoot palette 为 light 且 transcript 仍在

  @req:atc15
  场景: bad-keeps-old
    假如 当前为 dark
    当 apply 未知主题名
    那么 仍为 dark 且有诊断

  @req:atc16
  场景: select-applies
    假如 Themes 槽已打开
    当 选中 light 并确认
    那么 palette 为 light 且槽关闭

  @req:atc16
  场景: esc-keeps
    假如 Themes 槽已打开且当前 dark
    当 按 Esc
    那么 槽关闭且仍为 dark

  @req:atc16
  场景: no-auto-default
    假如 产品 host 默认构造
    当 观察主题探测状态
    那么 未启用 theme auto

  @req:atc17
  场景: border-follows-level
    假如 thinking level 为 high 且非 bash 前缀
    当 sync_editor_border 或等价
    那么 Editor 边框色为 Palette thinking_border 对应 high

  @req:atc17
  场景: bash-overrides-then-restore
    假如 thinking level 为 medium
    当 编辑器文本改为 !ls 再改回 hello
    那么 先为 bash 强调色，再恢复 medium thinking 边框

  @req:atc18
  场景: brand-line
    当 渲染 UiRoot
    那么 顶部卡片含 xylitol 且不含木糖醇与 ASCII logo

  @req:atc18
  场景: skills-all-visible
    假如 已加载多个长 skill 名
    当 窄宽渲染
    那么 所有 skill 名可见且无省略号

  @req:atc18
  场景: skills-above-scrollback
    假如 已加载 skills demo
    当 渲染 UiRoot
    那么 Skills 出现在 scrollback 之前

  @req:atc18
  场景: no-prompts
    假如 有 skills
    当 审查文案
    那么 不含 Prompts 或 prompt template 清单
