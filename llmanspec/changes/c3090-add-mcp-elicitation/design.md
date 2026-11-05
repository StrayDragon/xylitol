# Design — MCP elicitation

## 1. 入口

- McpClientHandler 实现 create_elicitation。
- form 模式：默认 decline，不弹 UI；URL 模式同样 decline。
- 不在日志中记录 message 内容或 schema 字段值；只记录 server / 字段名。

## 2. 秘密保护

- requestedSchema 的 properties 名若命中 password / secret / token / api_key / credential（大小写不敏感），直接 decline，不向用户展示。
- 表单不得收集秘密；本 change 只做拒绝，不做表单 UI。

## 3. 扩展点

- McpElicitationHandler trait 预留（name + handle），默认无 handler 即 decline。
- 不假设 UI 形状；真实 TUI 表单留给后续 change。

## 4. 测试

- 普通 form → decline。
- 含 secret 字段 → decline。
- 无 handler 时稳定返回 decline，不 panic。
