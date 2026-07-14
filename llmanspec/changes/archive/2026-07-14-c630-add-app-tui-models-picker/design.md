# Design — c630-add-app-tui-models-picker

## Decisions

| 主题 | 决议 |
|---|---|
| 命令名 | **`/model`**（对齐 pi）；**不**引入 `/models` |
| 无参 | 打开 `EditorSlot::Models` + 包 `SelectList`（替换 editor 槽） |
| 有参 `/model <id>` | 直接 `SetModel` / `select_model`，不开槽 |
| 过滤 | 槽内键入更新 filter；优先包 `fuzzy_match`，回退 `SelectList::set_filter` |
| busy | idle-only：busy 时短提示，不开槽 |
| 补全 | **不做**（→ c999） |
| CycleModel | 产品主路径退役；dispatch API 可保留但 TUI 无参不再调用 |

## Non-goals

- 内联 `/model ` 参数 CompletionSource（c999）
- Ctrl+Shift+M；运行时写 YAML
