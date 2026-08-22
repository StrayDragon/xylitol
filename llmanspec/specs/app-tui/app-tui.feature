# language: zh-CN
# managed by llman sdd partition-migrate
功能: app-tui

  @req:tui3
  场景: surfaces-retained
    当 检查 print 与 server 入口
    那么 两面仍存在且可分发

  @req:tui4
  场景: no-infra-import
    当 审查产品 TUI 依赖方向
    那么 无 agent 子模块内部或 infra 直达

  @req:tui5
  场景: driven-on-land
    当 变更完成检查新文件
    那么 均可从 tui::run 到达且无 allow dead_code 骨架

  @req:tui-index
  场景: split-capabilities
    当 列出 llmanspec/specs/app-tui-*
    那么 六个产品 capability 与 package-tui-testing 均存在
