# Tasks

- [ ] 1.1 `PointerPipeline` 增加 `forget_hover()`；`TUI` 增加 `clear_pointer_hover()` / `reprobe_pointer_hover()`
- [ ] 1.2 `refresh_pointer_hover_for_paint`：内容驱动的整帧重绘改为重探针；resize / overlay 才清空
- [ ] 1.3 `HostSession`：折叠布局变化时 `forget_hover`；`step_paint_only` 在命中区同步后重探针
- [ ] 1.4 harness 测试：流式整帧重绘后 hover 保持；折叠点击仍清除 hover
- [ ] 2.1 `paint_folded_streaming_thought`：流式 thinking 正文始终可见（含混合工具簇折叠态）
- [ ] 2.2 更新既有 activity-fold thinking 测试；新增思考结束后默认折叠测试
- [ ] 3.1 `rutis_error_sink`：`NotServedByName` 降级 debug，其余保持 error
- [ ] 3.2 单测：`NotServedByName` 无 ERROR、`PluginFailed` 仍 ERROR
- [ ] 4.1 specs：`app-tui-host` r3005 悬停稳定；`app-tui-transcript` r1351/r1355/r1364/r1372 流式 thinking 可见；`infra-observability` r3006 loader 提示降级
- [ ] 5.1 门禁：`just qa`
- [ ] 5.2 门禁：`just test-tui-e2e`
- [ ] 5.3 收口校验：`llman-sdd validate --all --strict`
