# Design — c1145-update-runtime-model-thinking-levels

## 现状（代码事实）

| 面 | 现状 |
|---|---|
| `ThinkingLevel` | `Off…High`，无 xhigh/max |
| `XyModelMeta.thinking_levels` | 字段在；`resolve_model_meta` 恒 `Vec::new()` |
| `ModelEntry` | 仅 `thinking: bool`，无 levels 列表 |
| `Settings.default_thinking_level` | 已有 `Option<String>`，选模未强制采用 |
| `ModelManager::set_thinking_level` | 只赋值；`thinking_level()` 仅按 `thinking` bool clamp 到 Off |

## 配置语义（对齐 pi 精神，YAML 面简化）

```yaml
models:
  models:
    my-claude:
      provider: anthropic
      model: claude-…
      thinking: true
      thinking_levels: [off, low, medium, high, xhigh]  # 可选；可含洞
```

| 配置 | 解析后 `XyModelMeta.thinking_levels` |
|---|---|
| `thinking: false` | `["off"]`（或空；运行时仅允许 Off） |
| `thinking: true` 且无 `thinking_levels` | `["off","minimal","low","medium","high"]` |
| 显式 `thinking_levels` | 原样（小写规范化）；未知名 **配置加载失败** 或跳过并 WARN——钉：**加载失败**（严格） |

`xhigh`/`max` 仅当显式写入列表（或日后 catalog 注入）才出现——与 pi「extended levels opt-in」一致。

## 运行时策略

1. **SetThinkingLevel**：level ∉ 当前模型支持集 → **拒绝**（Driver/dispatch 错误；不改值；不发成功事件）
2. **换模 / 启动**：当前 level ∉ 新支持集 → **clamp** 到默认：`default_thinking_level`（若合法）否则 `Medium`（若在集）否则集内最高非 Off 或 Off
3. **cycle**（可选本变更）：在支持集顺序上 `next`，环回到首项

## 与包边框

- 包 `ThinkingBorderLevel` 已有 7 档；产品映射在 c1150（名对齐即可）
- **MUST NOT** domain 依赖 xylitol-tui

## UX 边界

- 本变更无 UI。产品静默 vs demo 上行 → **c1150 purpose**。

## 后续（deferred）

- Provider 请求体 effort 映射：用户配置表（pi `thinkingLevelMap`）→ **c1165**；等更多 provider 接入后再升格。
