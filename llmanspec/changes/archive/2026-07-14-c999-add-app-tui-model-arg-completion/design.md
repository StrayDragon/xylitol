# Design — c999-add-app-tui-model-arg-completion

## Package merge（已合入）

| Commit | 内容 |
|---|---|
| `7208d0a` | `SlashArgCompletionSource` + 测试；**默认** bare `/model` 不 probe |
| `61c5783` | 空格后 auto-open；demo `with_bare_command(true)`；slash→arg hand-off |

产品跟包默认：**无 bare**。demo 可 bare 开 catalog；产品 bare Enter → c630 槽。

## Product wiring

```
set_completion_sources([
  SlashArgCompletionSource::new("model", catalog).with_id("model-id"),  // no bare
  SlashCommandSource::new([exit, model]),
])
```

- `/model`（无空格）→ slash 列表或无 popup；Enter 提交 → `OpenModels`
- `/model ` / `/model dee` → arg popup；Tab/Enter apply id → 仍是 editor 文本；再 Enter → `SetModel`

## Catalog refresh

`available_models()` → `(id, provider)`；`UiRoot::set_model_arg_catalog` 重建 sources（保留 slash 命令表）。

## Non-goals

- 改包 API；产品 bare catalog；c635 树 filter
