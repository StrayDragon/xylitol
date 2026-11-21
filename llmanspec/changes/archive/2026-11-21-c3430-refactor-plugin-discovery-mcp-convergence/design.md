# Design：插件单一脊柱

真源：c3410 design.md D7 / §4.5；本文只写本 change 特有形状。

## describe 清单 v2

{ name, commands?: [...], mcp?: { command, args, env? } }——mcp 字段存在即声明该二进制可作 stdio MCP server。

## 装配与信任门

- 发现（plugins/bin 扫描）→ 清单解析 → 两种接线：commands → CLI 委派（现状保留）；mcp → 待批准注册表。
- 首用批准：首次会话准备纳入其工具前，TUI ChoicePrompt（对齐 app-tui-trust 先例）确认；批准状态持久化（路径【细化】，候选 ~/.xylitol/plugins/approved.json）。
- config plugins.disabled / plugins.enabled 覆盖：disabled 优先级最高；enabled 语义【细化】。

## 退役清单

external_plugins 配置段、stdio JSON-RPC 桥、{plugin}__{tool} 命名、桥级 trust 语义；bun-demo 示例 MCP 化或移除【细化】。thoughtdag 转嵌入端：其 describe / install.sh 在其仓退役（跨仓任务）。

## seam

infra-plugin / cli-plugin-command / infra-mcp BDD 场景对齐 + 发现单测（tempdir fixture，test-infra 先例）。

## specs / BDD 绑定方案

- infra-plugin 新规则（清单 mcp 声明 / 首用批准 / disabled 覆盖，c3410 §5.5）场景 step 落地；r2991 / r2992 删除后其 bindings 随删（预期红名单项）。
- cli-plugin-command（r3358-r3362）与 infra-mcp（r1445-r1454）引用核查随触清；委派场景回归。
