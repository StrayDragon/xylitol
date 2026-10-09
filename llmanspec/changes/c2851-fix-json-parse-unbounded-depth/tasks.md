# Tasks

## T1 — 深度探测 + 上限常量
- [ ] `codec.rs`：`JSON_DEPTH_LIMIT = 2048` + std 单趟嵌套深度探测（转义感知、跳过字符串字面量内的括号）；两处 `disable_recursion_limit` 调用点前置探测超限早退。
- [ ] 单测：>2048 层嵌套在两入口按降级语义收口；~200 层深树通过。

## T2 — spec 锚点
- [ ] r1927 verified-by 由 `src/protocol/wire/codec.rs` 路径锚改指深度探测/回归测试 fn。

## T3 — 门禁收口
- [ ] fmt / clippy / `just qa` 绿 + `llman-sdd validate --strict` 0。
