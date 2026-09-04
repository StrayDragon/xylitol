# language: zh-CN
# capability: app-tui
# purpose: 跨切面不变量索引。产品细节见 app-tui-*；引擎/harness 见 package-tui-*；CLI 默认进 TUI 见 cli-entry。
# scope: src/, tests/

功能: app-tui

  @req:tui3 @human
  场景: three-surfaces-coexist
    - print（cli）、server（HTTP 监听器）与 tui（交互面）应用面 MUST 共存；不得以统一为名删除任一传输面。

  @req:tui4 @human
  场景: reuse-contract
    - 产品 TUI MUST 经协议契约、应用缝与 agent 公共入口导入，MUST NOT reach agent 子模块内部或 infra；MUST NOT 依赖已删除的独立 domain 顶栏。斜杠语义 MUST 复用 protocol::Command。

  @req:tui5 @human
  场景: no-skeleton-spray
    - 产品 TUI 面下每个新增源文件 MUST 在同一变更内由真实入口驱动；MUST NOT 用 #[allow(dead_code)] 落地未驱动骨架。死码分诊规则由架构 AGENTS 与 l8ng-audit-dead-code skill 承载。

  @req:tui-index @human
  场景: capability-index
    - 产品 TUI 行为细节 MUST 由 app-tui-host / app-tui-bridge / app-tui-transcript / app-tui-fixed-zone / app-tui-input / app-tui-commands 各自约束；引擎与四层 harness MUST 由 package-tui-*（含 package-tui-testing）约束，不得在本索引重复实现史细节。

  @req:tui-cs @human
  场景: TUI 只承担面本地
    - 产品 TUI MUST 只执行面本地能力（键盘、绘制、TTY、本机编辑器、剪贴板）。依赖工作区、模型、MCP 或会话生命周期的能力 MUST 由 host 角色执行。载体切分与同进程保留见 app-tui-bridge atb4。MUST NOT 把剪贴板等面本地能力交给远程 host 写入本机盘。
