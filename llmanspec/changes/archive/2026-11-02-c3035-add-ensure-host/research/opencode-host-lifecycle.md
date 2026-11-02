# OpenCode Host 生命周期对照（c3035）

一手来源：<https://opencode.ai/v2/docs/cli/>、Troubleshooting、`opencode serve` 文档；实例 idle 见 anomalyco/opencode#16616 / `experimental.location_idle_timeout`。

## 产品形态

OpenCode v2 默认**发现或拉起一个用户级共享后台服务**。本地客户端都连它。`--standalone` 才是私有实例；`--server URL` 连指定地址。`opencode serve` 是显式无头服务。

注册：`~/.local/state/opencode/service.json`。管理：`opencode service start|stop|status|restart`。`service set disabled true` 后 CLI 不再自动起共享服务。

## 和 xylitol 的差

| | OpenCode v2 | xylitol 现状 | c3035（B） |
|---|---|---|---|
| 默认 | 共享 daemon（近 C） | attach，未在听失败 | ensure + last-client |
| 私有实例 | `--standalone` | 无 | 不做换口 standalone（禁止 port+1） |
| 最后客户端断开 | **不杀**共享服务 | Host 本就常驻 | 自动拉起的实例杀；显式 serve 不杀 |
| idle | 卸 **location/Instance**（LSP/MCP，默认 60 分钟），不是整个进程 | 会话槽订阅宽限 ~60s | Host 进程 idle grace ~30s（仅 last-client） |
| 发现文件 | `service.json` | `~/.xylitol/serve.json` | 同一文件，加可选 `lifecycle` |

OpenCode 另有 `OPENCODE_IDLE_TIMEOUT`（serve 模式下 Instance 引用计数归零后约 5 分钟卸工作区）。那是工作区资源回收，不是「没 TUI 就退出进程」。不要把那层 idle 当成 B 的范本。

## 对 B→C 的含义

C 对齐 OpenCode 默认：自动拉起也 persistent，TUI 从不因零连接杀 Host。c3035 把 Host 做成「可 persistent 的独立进程 + 策略枚举」，C 只翻转缺省，不改 spawn / 注册 / attach。

本 change **不**引入 `service` 子命令家族；`serve stop` 做真即可。别名留给 C。
