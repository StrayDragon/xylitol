# crates/xylitol-rutis/

装配适配层（rutis 适配层）。仓库里唯一允许 `use rutis` 的包。

| 必须 | 不得 |
|---|---|
| 封装 `Ctx` / `Plugin` / 错误，给组合根用 | 被 `xylitol-runtime`、官方插件、`xylitol-app-*` 依赖 |
| 锁 rutis 0.13 线（workspace 依赖） | 认识信封；接线七个官方插件（那是组合根） |
| 组合根显式建至多一个 `Ctx` | 在 crate 加载时建 `Ctx`；每 token `require` / `parallel` |
| 未配置的 MCP / hooks 不装配 | 为空配置建 fiber 或外进程 |

本包不实现 ReAct 循环，不实现 hook 脚本协议。
