# Tasks

## 1. 合约

- [x] 1.1 `user-experience`：新增展示名 `XYlitol` vs 机器标识 `xylitol` 对照
- [x] 1.2 `app-tui-fixed-zone` r1223：头卡改为 `>_ XYlitol`；增场景「头部卡含产品名」
- [x] 1.3 `infra-provider`：UA 产品段 `XYlitol/<version>`（插件单测，不单独扩 BDD）
- [x] 1.4 `infra-otel`：缺省 `service.name` 为 `XYlitol`（单测，不单独扩 BDD）
- [x] 1.5 绑定分支后提交 specs

## 2. 对外指纹

- [x] 2.1 OpenAI 与 Anthropic `default_user_agent` 改为 `XYlitol/<version> (os; rust; arch)`；单测 `starts_with("XYlitol/")`
- [x] 2.2 OTEL 缺省 `service.name` 改为 `XYlitol`；显式配置仍覆盖；`example.yaml` 示例对齐；instrumentation scope 保持 `xylitol`

## 3. 展示面

- [x] 3.1 TUI 头卡标题与窄宽回落改为 `XYlitol`；harness / widget 单测按大小写断言品牌
- [x] 3.2 clap about 含 `XYlitol`；`name` 仍为 `xylitol`
- [x] 3.3 README 标题 / alt / 产品主语、`DESIGN.md` 系统名改为 `XYlitol`；命令示例仍 ``xylitol``
- [x] 3.4 根 `AGENTS.md` 产品定调首次对照：展示名 XYlitol vs 命令 / crate `xylitol`

## 4. 验证

- [x] 4.1 BDD：r1223 新场景绑定（卡含 `XYlitol` 且不含木糖醇）
- [x] 4.2 `just qa` 绿
