# Design — attach 模型同步 transient 化

## 决策表

| # | 决策 | 取舍 |
|---|---|---|
| D1 | transient 推送而非「仅在模型真变时 journal」 | 会话槽无「上次已 journal 模型」状态可查（journal 里的 ModelSelect 本就来自 attach 同步）；引入去重状态比去掉 journal 化更复杂。历史职责已在领域层 ModelChange。 |
| D2 | 复用 `push_resources` 的非 journal 推送形态（`downlink_server_request` 直推订阅者） | 同文件既有先例，语义「snapshot 类下行不消耗 seq」一致。 |
| D3 | 重复 transient ModelSelect 幂等收敛 | 客户端 `cached_model` 覆盖写；TUI 徽标 set 同值不闪变（既有 bridge 行为）。 |
| D4 | replay 消费者不再从 journal 学到模型 | 每个 bind 自带同步：late subscriber 在自身 bind 收到；长期会话回放 consumers（session export）用领域条目，不受影响。 |

## 失败模式

- 若写者恰在 bind 后、transient 推送前换模型：换模型走 set_model 写路径（成功应答携带新状态），下一次 bind 再收敛——不劣于现状。
