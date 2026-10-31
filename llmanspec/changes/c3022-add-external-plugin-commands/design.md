# 设计：外部插件命令面

## manifest

```json
{
  "protocol_version": 1,
  "name": "x-magpie",
  "capabilities": ["commands"],
  "commands": [{"name": "x-magpie", "description": "magpie 模型同步", "argument_hint": "refresh"}]
}
```

## 注册与寻址

- 命令名 = `plugin:{plugin-name}`；`plugin-name` 来自 `external_plugins[].name`。
- 描述取 manifest 第一条命令的 description；无则用默认文案。
- SDK 新增 `XyPluginCommand` trait：`name` / `description` / `invoke(args)`。
- `HostRegistries.plugin_commands: Registry<dyn XyPluginCommand>`。

## 调用

- `Command::PluginCommand { plugin, args }` → 驱动查找 `plugin:{plugin}` → `invoke(args)`。
- 外进程 RPC：`commands/call {args}` → `{text}`；超时/错误 → `Err`。
- 结果 `DispatchOutcome::PluginCommandOutput { plugin, text }`。

## TUI

- `PendingSlash::PluginCommand { plugin, args }`；解析 `/plugin:<name> [args]`。
- busy 拒绝（与 reload 同级）。
- 成功/失败均以滚动提示呈现（`push_scroll_notice`），失败前缀 `Error: `。
- 补全目录：静态产品命令 + `driver.get_commands()` 中 `plugin:` 前缀项。

## 失败隔离

- 插件未注册该命令 → 可观察错误；插件进程退出 → 错误；均不崩溃。
