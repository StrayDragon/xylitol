# Design: c3360-remove-duplicate-base-tools

## 目标

`install_product_tool_set` 对同一份工具集只调用一次 `set_base_tools`。

## 两种形状

| | 形状 A | 形状 B |
|---|---|---|
| 改动 | 删掉第二次调用 | 给 boot 期 setter 和会话期 setter 分成两个类型 |

## 选择

用形状 A。

`set_base_tools` 只做 `self.base_tools = Some(tools)`。第二次调用不改变工具集。boot 期和会话期的 setter 都仍是 `&mut self`。当前没有一条已写明的规则要求把它们分成两个类型。

## 完成时的读法

`compose_tools` 的结果仍先写入基座，再交给 `try_set_tools_inline`。`deslop-please` 里 `XyInProcessDriver::base_tools` 在这个函数里只剩一处调用。剩下的那一处 setter 留在基线里。
