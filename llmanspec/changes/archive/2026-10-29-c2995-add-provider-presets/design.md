# Design

## Context

`XyModelEntryConfig` 是 `models.<name>` 的唯一档案形状：`provider / model / base_url / api / compat / api_key / thinking / thinking_levels / context_window`。同一兼容端（如 OpenCode Zen）只是 `base_url` 与 `api` 不同，其余字段重复。issue #20 要的是「名字即用」，不是第二套内核。

## Decision

### 1. 预设形状

新增 `preset: Option<String>` 到 `XyModelEntryConfig`（`#[serde(default)]`，`deny_unknown_fields` 保持）。内置目录放 config 插件：

```
ProviderPreset {
  id: &'static str,
  kind: XyModelKind,
  api: Option<&'static str>,
  compat: Option<&'static str>,
  base_url: &'static str,
  default_model: Option<&'static str>,
  thinking: bool,
  thinking_levels: Option<&'static [&'static str]>,
  context_window: u64,
}
```

首版预设：`zmodel`、`opencode-zen`、`clinepass`、`commandcode`。预设只提供接入默认值，不提供密钥、不提供 system prompt、不改变工具/ReAct。

### 2. 解析优先级

```
preset 默认值 < models.<name> 显式字段 < execution / agents 覆盖
```

- `preset` 存在且未知 → 解析失败并给出「unknown preset: <id>」诊断；不得静默回落成裸档案。
- `model` 可由预设 `default_model` 提供；用户显式 `model` 覆盖。
- `api_key` 必须来自用户配置（`{{ secret.KEY }}` 或环境），预设不得内置密钥或自动读取任意环境变量。
- `thinking_levels` 若预设给出，用户显式列表覆盖；显式 `thinking: false` 仍优先。

### 3. 发现与文档

- `configs/example.yaml` 增加预设示例（注释形式，避免默认发起真实调用）。
- 文档表：预设名、兼容族、`api`、`compat`、默认模型、能力边界、已知可用基线。
- 不在 TUI 预埋第二套预设选择动作；`/model` 仍读已解析档案。

## Alternatives

- **B1. 在 `models` 顶层放 `presets` 映射**：用户仍需手写引用与覆盖，且与 `models.<name>` 档案重复；拒绝。
- **B2. 预设直接展开成完整 YAML**：无法表达「显式字段覆盖」，且容易复制密钥；拒绝。
- **B3. 运行时下载预设目录**：引入网络与版本漂移；首版不做。

## Risks

- 预设默认值可能与上游模型/端点变化漂移；文档须标注「已知可用」日期，测试只锁解析形状，不锁真实网关。
- 预设名成为产品词表；新增/改名须走 SDD，不得在配置里留别名双解析。
- `deny_unknown_fields` 下新增 `preset` 字段是向后兼容的（旧配置不写即可）。

## Test Seams

- 预设默认值 + 显式覆盖的解析结果。
- 未知预设诊断。
- 预设不携带密钥 / 不改变 `provider` 之外的 ReAct 形状。
- `configs/example.yaml` 生成脚本与 schema 同步。
