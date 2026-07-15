# Design — c996

## 发射点（唯一）

| API | 事件 | context |
|-----|------|---------|
| `select_model` 成功 | `model_select` | model, previous, source=`set` |
| `cycle_model` 成功 | `model_select` | model, previous, source=`cycle` |
| `set_thinking_level` | `thinking_level_select` | level, previous |

语义：observe + fail-open（Blocked 只 warn）。

## BDD

扩展 `run_wiring_operation`：`选择模型 fake`、`设置思考级别 high`；启用 hooks-wiring 观察场景（或场景大纲例子行）。
