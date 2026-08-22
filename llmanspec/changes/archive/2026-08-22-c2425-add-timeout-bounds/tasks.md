# Tasks

- [x] t1: agent-tools t24/t25 改写 + doc 场景同步 + 新增 fs 四工具有界 req；infra-bash be8 改写
- [x] t2: package-ai-bridge 新增 HTTP connect/idle/total 有界 req；infra-mcp 新增调用期超时 req
- [x] t3: app-tui-host 新增 unary 请求级有界 + mux keepalive 检测两 req
- [x] t4: 五 capability strict validate；全量 validate 收口（BDD 对现行代码应保持绿——本票仅合约先行）
- [x] t5: 工具层默认上限——bash/grep/find omit→120s、read/write/edit/ls 30s 包裹，schema 文案与单测同步
- [x] t6: hooks 未配置 timeout_secs 默认 30s + 单测
- [x] t7: MCP call_tool/list_all_tools 调用期 120s 超时 + 单测
- [x] t8: bridge provider HTTP connect 10s / SSE chunk-gap idle 90s / 非流式 total 300s + 单测
- [x] t9: http_ws unary 请求级超时（常规 30s、reload 类分级）+ mux keepalive 半开检测；驱动级测试
