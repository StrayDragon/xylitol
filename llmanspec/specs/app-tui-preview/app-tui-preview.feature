# language: zh-CN
# purpose: 产品 TUI designing 组合：同一套 client 挂假 Host，场景槽切命名可达态；入口是交互表面的 debug 叶 tui designing，不是顶层表面，不是网页第二套 UI。designing preview（预览设计）= 把某场景注入同一 TUI 画布。

功能: app-tui-preview

  @req:r2906
  规则: tui-designing-debug-entry
    designing 是交互表面的 debug 叶，不是顶层表面动词。debug 构建 须提供 xylitol tui designing：打开产品 TUI designing 组合（假 Host + 场景槽）。release 构建 不得 列出 designing，xylitol tui designing 须按未知子命令失败。不得提供顶层 preview 或 designing；不得提供 tui --designing 旗标；不得放进 serve 子树。TTY 裸跑 不得进入 designing。designing 不得 因本机 Host 未在听而失败。

    场景: debug-tui-help-lists-designing
      假如 CLI 已解析且为 debug 构建
      当 xylitol tui --help
      那么 子命令含 designing

    场景: release-tui-help-omits-designing
      假如 CLI 已解析且为 release 构建
      当 xylitol tui --help
      那么 子命令不含 designing

    场景: help-omits-toplevel-designing
      假如 CLI 已解析
      当 xylitol --help
      那么 Commands 不含 preview 与 designing

    场景: designing-does-not-need-host-listener
      假如 本机 Host 未在听且为 debug 构建
      当 xylitol tui designing
      那么 进入 designing 组合且不得因未在听非零退出

  @req:r2907
  规则: designing-scene-slot
    designing 组合打开后 必须提供场景槽（固定区「槽」，不得做成常驻底栏 debug 条）：列出场景 id 与描述；选中后 必须关闭场景槽并将该场景注入同一 TUI 画布（designing preview）。切场景 必须走槽，不得依赖斜杠 /debug，也不得把场景 id 当斜杠参数注入。关槽后须经斜杠 /designing 再打开场景槽（只开槽）。/designing 仅 designing 组合解析与列出，须置顶于 SlashCommandSource，键入 / 后补全列表须可见该条；不得为此新增专用快捷键。产品 TUI（非 designing 组合）不得出现该槽，不得解析 /designing（未知斜杠）。

    场景: designing-catalog-lists-scenes
      当 打开 designing 组合并打开场景槽
      那么 槽内列出场景 id 与描述

    场景: selecting-scene-injects-canvas
      当 在场景槽选中 "session-tree-multiturn"
      那么 画布为该场景且不再停留在空会话

    场景: selecting-another-scene-switches-canvas
      当 在场景槽选中 "session-tree-multiturn"
      当 再打开场景槽
      当 在场景槽选中 "session-tree-labeled"
      那么 画布为后一场景

    场景: closed-scene-slot-reopens
      当 打开 designing 组合并打开场景槽
      当 关闭场景槽
      当 再打开场景槽
      那么 槽内列出场景 id 与描述

  @req:r2908
  规则: designing-preview-isolation
    designing preview 必须新建隔离会话，不得覆写用户装载前的当前 session 文件内容。未知场景 必须短错误提示且不得崩溃。注入 必须在本进程完成。

    场景: unknown-scene-errors-without-crash
      当 designing 请求未知场景
      那么 提示错误且进程不崩溃

  @req:r2909
  规则: designing-not-on-wire
    装载场景 不得出现在跨进程信封或远程 unary（不进 protocol::Command 公开面）。远程 TUI attach 真 Host 时 不得看到 designing 场景表。

    场景: designing-inject-absent-from-wire-commands
      当 枚举跨进程公开命令
      那么 不含场景装载命令
