# Tasks: c2705-refactor-runtime-actor-api

> pre-start。依赖 `c2700` 归档或本分支已落地其可见性。不 `change start` 直到认领。

## 1. 闸

- [ ] 1.1 确认 `c2700` 已使 `xylitol::agent` 对外部不可达。
- [ ] 1.2 列出 Driver / embed / server 对 `AgentCapabilities` 与 `runtime.inner` 的调用清单。

## 2. Runtime 收口

- [ ] 2.1 [blocked-by: 1.2] 为清单中的 Capabilities 调用补 `AgentRuntime` 方法；实现只转内部 caps。
- [ ] 2.2 [blocked-by: 2.1] in-process Driver 改为只持 Runtime；删除对 Capabilities 的字段/re-export。
- [ ] 2.3 [blocked-by: 2.2] `AgentCapabilities` `pub(crate)`；清 `agent/mod.rs` / embed rustdoc。
- [ ] 2.4 [blocked-by: 2.3] app 测试不再 `use …capabilities::AgentCapabilities`（agent 单测除外）。

## 3. 验证

- [ ] 3.1 [blocked-by: 2.4] 相关 unit + Driver 集成测。
- [ ] 3.2 [blocked-by: 3.1] `just qa`；`llman sdd validate c2705-refactor-runtime-actor-api --strict --no-check`。
