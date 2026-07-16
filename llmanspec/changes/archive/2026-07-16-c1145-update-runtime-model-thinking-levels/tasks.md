# Tasks — c1145-update-runtime-model-thinking-levels

## 1. Domain 枚举

- [x] 1.1 `ThinkingLevel` 增加 `Xhigh` / `Max`；`as_str` / serde / 单测
- [x] 1.2 `parse`（或 FromStr）覆盖 off…max；未知返回 None

## 2. 配置 → meta

- [x] 2.1 `ModelEntry.thinking_levels: Option<Vec<String>>`（或 `Vec`+缺省）
- [x] 2.2 `resolve_model_meta`：按 design 表填 `XyModelMeta.thinking_levels`；非法名加载失败
- [x] 2.3 配置/解析单测：缺省标准档、显式含洞、thinking:false

## 3. Manager / Driver

- [x] 3.1 支持集校验：`set_thinking_level` 拒绝不支持
- [x] 3.2 `select_model`（及启动）clamp + 可选采用 `default_thinking_level`
- [x] 3.3（可选）`cycle_thinking_level` 仅在支持集内

## 4. 校验

- [x] 4.1 `LLMANSPEC_BASE_REF=main llman sdd validate c1145-… --strict --no-interactive`
- [x] 4.2 相关 unit / BDD 绿；`just lint`
