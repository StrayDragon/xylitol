# Design — MCP elicitation form UI

## 1. schema 子集

- MCP `requestedSchema` 只支持 primitive：`String` / `Number` / `Integer` / `Boolean` / `Enum`。
- 支持 title / description / default / required；string 支持 minLength / maxLength / format（email / uri / date / date-time）；number/integer 支持 minimum / maximum。
- 嵌套 object / array 不在 MCP 子集内；遇到未知形状整表 decline 并诊断。
- 表单只渲染当前 schema 支持的字段；必填缺失时 accept 禁用。

## 2. 秘密策略

- 字段名命中 password / secret / token / api_key / apikey / credential / private_key（大小写不敏感）→ 整表 decline，不渲染任何字段。
- URL 模式仍 decline；不做浏览器自动化。
- 不记录 message / 字段值；只记录字段名。

## 3. 流程

- `create_elicitation` 不再立即 decline；向 host 发 elicitation 请求事件（server / message / schema / request id）。
- TUI 打开表单槽位；用户 accept / decline / cancel。
- 响应经 unary 回 host；`create_elicitation` 等待 oneshot，超时 5 分钟按 decline。
- 无 TUI（headless / print）或槽位不可用时按 decline，不阻塞 agent。
- accept：校验类型/必填后构造 `ElicitResult { action: Accept, content: Value }`；decline/cancel 不带 content。

## 4. MRTR

- `ElicitResult.content` 必须符合 schema；server 据此继续 MRTR 重试。
- 不接受 server 在 schema 外追加字段；未知字段忽略并诊断。
- 一次 elicitation 只允许一个响应；重复响应丢弃。

## 5. 审计

- 结构化日志：server、字段名列表、action、reason、timestamp。
- 不写值、message、schema 原文；不写 session 历史。
- 审计失败不阻塞响应。

## 6. TUI

- 表单槽位：字段列表、焦点、错误提示、accept / decline / cancel。
- 秘密字段不显示；不支持 schema 显示 decline 原因。
- 取消/超时走通知条或滚动提示；不泄漏值。

## 7. 测试

- 单测：schema 子集、secret 命中、accept/decline/cancel、超时、重复响应、审计脱敏。
- BDD：fixture 发起 form → TUI 填写 → accept → server 收到 content；secret → decline。
- 门禁：`just qa` / `just qa-e2e`。
