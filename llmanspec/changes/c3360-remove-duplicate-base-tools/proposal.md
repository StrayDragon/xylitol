---
depends_on: []
title: 装配工具集时只写入一次基座工具
needs_specs_change: false
---

# 装配工具集时只写入一次基座工具

## Why

`set_base_tools` 只做 `self.base_tools = Some(tools)`。`install_product_tool_set` 对同一份 `tools.clone()` 连续调用了两次。第二次不改变工具集，也不表达顺序。

boot 期 setter 和会话期 setter 仍然都是 `&mut self`。这次不给它们分类型。当前没有一条已写明的规则被这个类型缺口打破。

## What Changes

删掉第二次 `set_base_tools`。`compose_tools` 的结果仍先写入基座，再交给 `try_set_tools_inline`。

## Capabilities

- 工具装配的既有行为不改。不改规格文本。

## Impact

- 代码：`crates/xylitol-kernel/src/driver/in_process/mod.rs` 一处调用。
- 验证：现有装配测试保持通过。不新增 BDD 场景。
- 与已有草案无文件重叠。无 `depends_on`。
