# hooks（pi 式 Pre/Post 脚本钩子）→ rutis 事件模型映射备忘

一行指针级，rutis P1 立项时展开。锚定：`~/Projects/__straydragon__/rutis/integrated-xylitol-suggestions.md` §3.1/§4-P1、`~/Projects/__straydragon__/rutis/docs/design-philosophy.en.md`。

| xylitol 现行（agent-hooks r1-r3 时代） | rutis 对应 | 语义差 |
|---|---|---|
| `HookDispatcher::dispatch`（进程内枚举事件） | 事件派发（`emit`） | rutis 无集中 dispatcher，订阅制 |
| hook 脚本经 stdin JSON 上下文 | 进程内订阅者直调 / 跨进程走 bridge | 同进程无序列化边界 |
| stdout `block`/`allow` 短路 | `bail`（短路）/ `waterfall`(链式改写) | modify 语义对应 waterfall；**跨进程只通知**（boundary §5.8） |
| 三层配置合并（global/project/user） | 插件注册序 + 实例作用域 | 合并语义须宿主层自建 |
| `XyHookBus` 精选导出（test-hooks-wiring r1819） | 服务按名解析 | 门面词表不变，实现换内核 |

重审判据：r1-r3 场景验证的是「机制形状」（谁调谁、数据怎么流），rutis P1 落地后这些形状改变——届时按映射表逐条改写或退役；行为级承诺（hook 可取消/可修改、超时上界、配置覆盖序）不变，由 test-hooks-wiring 矩阵与 agent-hooks 行为场景继续守护。
