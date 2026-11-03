# 设计：产品身份 XYlitol

## 两套名字

| 层 | 字面量 | 用途 |
|---|---|---|
| 产品展示 / 对外指纹 | `XYlitol` | TUI 头卡、README 标题、CLI about、HTTP UA 产品段、OTEL 缺省 `service.name` |
| 机器标识 | `xylitol` / `XYLITOL_*` | 命令、crate、路径、环境变量、日志 target、instrumentation scope |

XY 是探索坐标的品牌拼法，不是 Title Case。网关用大小写不敏感匹配仍能对上 `xylitol`；大小写敏感前缀 `xylitol/` 会错过，这是有意的一次性切换（预发布，尚无外部检测规则）。

## 字面量落点

不抽 sdk 品牌模块（sdk 是词表 / 端口，不是产品名）。四处各钉同一字面量，单测锁死：

```text
TUI 头卡     xylitol-app-tui loaded-resources title
UA           provider-openai / provider-anthropic default_user_agent
OTEL 缺省    official-plugin-obs service.name unwrap_or
CLI about    根包 clap
```

后人改名：搜 `XYlitol`。禁止为旧小写 UA 留兼容别名。

## OTEL 分层

- `service.name` 缺省 = 产品身份 `XYlitol`（看板 / Langfuse 服务名）。
- instrumentation scope 仍 `xylitol`（库 / crate 身份）。
- 用户配置了 `otel.service_name` 则原样使用，缺省路径才写 `XYlitol`。
- 预发布不迁历史时序。

## TUI

头卡保持 Codex 风 `>_ ` 前缀，后面换成 `XYlitol`。`xylitol` 与 `XYlitol` 都是 7 列，窄宽截断不必改布局。禁止木糖醇中文标签与 ASCII logo 不变。

## 文档

README / DESIGN / 根 AGENTS：产品当主语时用 `XYlitol`；命令、路径、crate 仍反引号 `xylitol`。不要把机器标识改成混合大小写。
