# Tasks: c1510-fix-tui-streaming-assistant-paint

## Specs

- [x] `app-tui-host` 新增 ath26 + feature 场景；显式声明 Ctrl+O 合约不变
- [x] `llman sdd change attach` + `validate --strict`

## Paint

- [x] `StreamingAssistantPaint`（或等价）+ `find_stable_markdown_prefix_end`
- [x] `render_scrollback` assistant streaming 分支接线；width/fold/theme 失效
- [x] UiRoot 持有并在 apply/theme 路径正确 invalidate

## Verify

- [x] Harness：长流式 TextDelta → full parse 上界；与全量解析行一致
- [x] 既有 Ctrl+O / expandable / ath25 测绿灯
- [x] `just test-tui`（相关）绿
