# Tasks：插件单一脊柱

> 依赖 change：c3410——假设其特性已合入基线。needs_specs_change: false。
> seam：infra-plugin / cli-plugin-command / infra-mcp BDD + 发现单测（tempdir fixture，test-infra 先例）。

- [ ] T1: 前置自检（强制门）——c3410 信任门与桥退役条款已合入基线；缺失 → STOP 并报告。
- [ ] T2: describe 清单 v2——schema（commands + mcp{command, args, env?}）+ 解析校验 + 单测。[blocked-by: T1]
- [ ] T3: 自动注册——发现 → MCP server 接线经既有 mcp 插件管线；首用批准流（ChoicePrompt 对齐 + 批准持久化）；config disabled 优先级最高。[blocked-by: T2]
- [ ] T4: 私有桥退役——external_plugins 配置段、stdio JSON-RPC 桥、{plugin}__ 命名删除；全仓编译绿。[blocked-by: T1]
- [ ] T5: 委派保留核查——cli-plugin-command 委派机制回归（surface 委派语义不变）。[blocked-by: T3, T4]
- [ ] T6: 示例与文档——bun-demo MCP 化或移除【细化】；docs/plugin-command.md 更新。[blocked-by: T5]
- [ ] T7: 全量验证——nextest + just qa + 相关 BDD 对齐。[blocked-by: T5, T6]
