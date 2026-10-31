# Tasks

## 1. 规则
- [x] infra-plugin r3027
- [x] app-tui-commands r1206
- [x] protocol-app 规则

## 2. SDK / 协议
- [x] `XyPluginCommand` trait 与 `HostRegistries.plugin_commands`
- [x] `Command::PluginCommand` 与 `plugin_command` 方法

## 3. 内核
- [x] manifest `commands` 解析与注册
- [x] `commands/call` 调用与失败隔离
- [x] `get_commands` 合并插件命令
- [x] dispatch 分支与 outcome

## 4. TUI
- [x] 解析 `/plugin:*`、busy 拒绝
- [x] dispatch → 滚动提示
- [x] 补全目录合并
- [x] harness 测试

## 5. 门禁
- [x] 相关包测试绿
- [x] `just qa` 绿
