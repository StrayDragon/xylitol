# delayed-changes / next-todo：本波吸收什么

对照 `llmanspec/delayed-changes/next-todo/`。那些提案仍 **park**，不升格、不拷进 `changes/`。
c2805 只吸收「模型怎么看见 Todo」这一刀，用来砍掉 `todo_list` 和额外注入通道。

## 吸收（写进本 change design / specs）

| 来源 | 取 | 不取 |
|---|---|---|
| c1895 Q10 | Todo 驱动注意力；栏不是 Todo SSOT；coding 默认不做 always-on 仪表盘 | Runtime 列、clock、tool_calls、profile、token 预算、`AgentStatusBar` kind |
| c1895 Q6′ | 出站 generate 前让模型看见当前态；**不要** refresh 工具 | 每轮盲目尾插并 **persist** 栏消息 |
| c1896 | Todo 对模型可见，走 typed 投影，不塞进 system / Runtime KV | Agent 列 API、`status_bar_publish`、kind 注册表 |
| c1897 / c1898 | 不需要：本波投影 **不落盘**，无堆积、无 refresh | compact 保栏策略、`statusline_refresh` |

## 注入形态（本波钉）

三条缝见 `design.md`「模型怎么看见清单」：A 稀疏落盘 / B 请求时投影 / C TUI 固定区。

- **真源**仍是 `agent_todo` Custom latest-wins（不进 `as_agent_message`）。
- **B**：`AgentStatusBar::render` 至多一条 user 行；清单在 `<todo>`；空表省略。
- **不**写入 transcript，故不占 A（`session_env`）、不触发 c1897。
- **不**进 system 前缀（r1015 / r1018 不动）。
- 后续读数：`AgentStatusBar` 加字段 + 根下新子标签，不是第二条 user 行。

## 仍 park

c1895 全栏子系统、c1896 通道壳、c1897 压缩保栏、c1898 refresh、c1910 工具结果冻结。
Todo 产品面不再等这些 change。
