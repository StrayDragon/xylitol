# Tasks — c3031

## 1. Specs

- [x] 1.1 改 `app-tui-transcript`：att8 / att21 / att25 / att27 去掉「流式必须展开」，改为默认展开、用户可折、增量不丢。
- [x] 1.2 在 `activity-live-window` 增加可执行场景：挂载流式思考 → 点块头收起 → 再追加增量仍收起。

## 2. 呈现

- [x] 2.1 `thinking_effective("live:streaming")` 无覆盖时为 true；paint 与 toggle 共用。
- [x] 2.2 `paint_folded_streaming_thought` 与仅 thinking 的 live 簇头按有效态画正文/三角，目标为 `Thinking("live:streaming")`。
- [x] 2.3 ActivityFold 关闭时块级折叠同样生效。
- [x] 2.4 `apply_ui_model` 在流式 id 落地时迁移覆盖；仅 thinking 且用户展开时打开簇。

## 3. 测试

- [x] 3.1 单测：默认可见；点击收起；再 delta 不展开；再点展开得完整缓冲；未点击则结束后默认收起；点击展开则结束后仍展开。
- [x] 3.2 混合簇：簇折叠时 Thinking 行仍在；点 Thinking 行收起正文；不误开工具细账。
- [x] 3.3 BDD 场景绑定并通过。

## 4. 验收

- [x] 4.1 相关单测与 BDD 绿。
- [x] 4.2 `llman-sdd validate c3031-update-streaming-thinking-fold` 结构通过。
