# Tasks: c1540-update-tool-timeout-unlimited

## 1. 类型与合约

- [x] 1.1 引入 `ToolTimeout`（或等价 `Option<Duration>` 封装）于 protocol/infra 合适落点；`from_secs_opt`：`None`→Unlimited，`0`/非法/超 max→Err
- [x] 1.2 live specs：`agent-tools` / `agent-hooks` / `infra-bash` 按本 change 更新；feature 场景挂 `@req`
- [x] 1.3 `llman sdd validate c1540-update-tool-timeout-unlimited --strict --no-check`

## 2. bash 统一

- [x] 2.1 `BashArgs.timeout` 改为 `Option`；默认 Unlimited；schema description 对齐
- [x] 2.2 `execute_streaming` 传入并尊重 timeout；`InfraBashExecutor` / `BashExecOpts` 去掉硬编码 30s
- [x] 2.3 非流式路径共用同一解析；有限时时保持 graduated SIGTERM→SIGKILL（`b3`）
- [x] 2.4 单测：显式 1s 超时；省略 = Unlimited；流式不丢 timeout；`timeout=0` 拒绝

## 3. grep / find

- [x] 3.1 Args + schema 可选 `timeout`；默认 Unlimited；有限时杀进程并 `XyToolError::Timeout`
- [x] 3.2 单测 / BDD 场景

## 4. Hook

- [x] 4.1 配置 `timeout_secs: Option<u64>` 默认 `None`；dispatcher 去掉对缺省的 `max(1)`
- [x] 4.2 显式超时仍杀脚本（既有 hook-timeout 场景）；缺省不因 timer 杀
- [x] 4.3 更新 example / 文档中「默认 5s」表述（若有）

## 5. 校验

- [x] 5.1 相关 `cargo test`（tools / hooks / bash_exec）
- [x] 5.2 `just fmt` + 相关 clippy
- [x] 5.3 `llman sdd validate c1540-update-tool-timeout-unlimited --strict --no-check`
