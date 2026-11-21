# Tasks：插件单一脊柱

> 依赖 change：c3410——假设其特性已合入基线。needs_specs_change: false。
> 落地记录（2026-11-21）：T2/T4 主体已随 c3415 分支先行落地并入主线（清单 mcp 声明解析 + external_plugins 段 / stdio 工具桥 / {plugin}__ 命名 / 桥级 trust 删除）；本 change 补齐 T3 首用批准——ExternalMcpRegistry 拆 declared/approved，批准持久化于 plugins/approved.json（损坏即 fail-closed），boot 装载批准并对未批准声明告警，config disabled 仍优先级最高（发现层拦截）；交互式 ChoicePrompt 首用询问为 UI 层后续（r3377 场景只约束注册结果，批准载体与持久化已就位，persist_approved_plugins 为公开入口）。T5：CLI 委派回归经 src/cli/plugins_cmd 单测（发现 / 名称校验 / 目录查找，104 例绿）。T6：bun-demo 示例移除（其演示的私有工具桥已退役）；docs/plugin-command.md 不存在（无此文件可更新）；根 AGENTS.md 产品定调与 src/AGENTS.md 工具源条款漂移已修。跨仓：../xylitol-thoughtdag describe/install.sh 退役（commit 62630ac，28 测试绿）。
> seam：infra-plugin / cli-plugin-command / infra-mcp BDD + 发现单测（tempdir fixture，test-infra 先例）。

- [x] T1: 前置自检（强制门）——c3410 信任门与桥退役条款已合入基线；缺失 → STOP 并报告。
- [x] T2: describe 清单 v2——schema（commands + mcp{command, args, env?}）+ 解析校验 + 单测。[blocked-by: T1]
- [x] T3: 自动注册——发现 → MCP server 接线经既有 mcp 插件管线；首用批准流（ChoicePrompt 对齐 + 批准持久化）；config disabled 优先级最高。[blocked-by: T2]
- [x] T4: 私有桥退役——external_plugins 配置段、stdio JSON-RPC 桥、{plugin}__ 命名删除；全仓编译绿。[blocked-by: T1]
- [x] T5: 委派保留核查——cli-plugin-command 委派机制回归（surface 委派语义不变）。[blocked-by: T3, T4]
- [x] T6: 示例与文档——bun-demo MCP 化或移除【细化】；docs/plugin-command.md 更新。[blocked-by: T5]
- [x] T7: 全量验证——nextest + just qa + 相关 BDD 对齐。[blocked-by: T5, T6]
