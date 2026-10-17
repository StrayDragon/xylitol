---
brief: "日志与 provider-trace 窄读排障（obs-* 子命令 / spinner 卡住 / 滚轮延迟 / RUST_LOG / XYLITOL_DEBUG / Langfuse / OTLP）"
---

# 观测 / Inspect（省 token）

**边界**：窄读与分组摘要。日志开关 / schema SSOT → `src/AGENTS.md`（Provider / 观测）与 `app/cli/logging.rs`。
**产品前瞻**：进程内底座在 `docs/architecture/进程内观测.md`。未兑现观测方向（infra lane / 采样）不在仓内维护，见 GitHub feature request。

## 硬约束（agent）

1. **MUST** 用 `just obs-*` 或 `scripts/inspect_provider_trace.py`；**MUST NOT** 整文件 `Read` JSONL / 大 log。
2. 单次工具输出 **≤ ~40 行**；再深挖时换子命令 + 过滤。
3. 结论只引用：`request_id`、`turn_id`、`kind`、`event`/`variant`、`lag_ms`、lifecycle 名；**勿**贴完整 SSE 正文。
4. 路径用下方**发现规则**，勿在回复 / 新文档里写死某一用户 home。

## 路径发现（勿写死）

运行时文件在 **`{agent_dir}/logs/`**：

| 文件 | 写入方 |
|------|--------|
| `provider-trace.jsonl` | fastrace `FileTraceReporter`（与 `log` 无关） |
| `xylitol.log` | `env_logger` 文件 sink（`init_logging`） |

**解析 `agent_dir`（按优先级）**：

1. 用户 / 会话已给出的 agent 数据目录
2. 自定义 `HOME`（E2E 常用临时 `HOME`）
3. 代码默认：`DefaultResourceLoader::default_agent_dir()` → `$HOME/.xylitol`（实现见 `infra/resource`；当前产品代码不读取 `XYLITOL_AGENT_DIR`）

```bash
# 示例：默认布局（仅当未另指定 agent_dir）
LOG_DIR="$HOME/.xylitol/logs"
# 有自定义 agent_dir 时：LOG_DIR="$AGENT_DIR/logs"
# 若 E2E 使用了临时 HOME，则按该 HOME 计算，例如：LOG_DIR="$TMP_HOME/.xylitol/logs"
```

`inspect_provider_trace.py` 默认同上；覆盖：`--path "$LOG_DIR/provider-trace.jsonl"`。

> 注：`just obs-tui-lag` 的 shell 封装额外接受 `XYLITOL_AGENT_DIR` 作为脚本默认值覆盖；这不是产品代码的 agent_dir 解析，仅为该 just 命令的便捷参数。

**日志开关**（`logging.rs`）：debug 构建默认开 log+trace；release 需 `RUST_LOG` / `XYLITOL_DEBUG=1`，或仅 `XYLITOL_PROVIDER_TRACE=1`（可只开 trace、不开级别日志）。**永不**写 stdout/stderr（保 TUI）。

## 健康检查（空 log / 双通道）

`trace` 与 `xylitol.log` **独立**：一边增长、另一边为空**不一定是产品 bug**。

| 现象 | 先查 | 常见原因 |
|------|------|----------|
| log **0 字节**，trace 有数据 | `wc -c` / `stat` 两文件；是否刚被截断 | 人工/脚本 `>` 清空；或只开了 provider-trace |
| log **不存在**，trace 有 | 日志开关 | release + 仅 `XYLITOL_PROVIDER_TRACE=1`（`want_log=false` 不建 log） |
| 两文件都不长 | `summary` | 观测未开；或写到**另一** `HOME`/`agent_dir`（测例临时目录） |
| log 存在但不增长 | 复现一轮 + `tail` | `env_logger::try_init` 失败时文件可已创建却无常规写入——应出现 `env_logger init failed` 面包屑行（见 `logging.rs`） |

复验级别日志：开启后跑一轮短对话，再 `tail -n 20 "$LOG_DIR/xylitol.log"`（仍勿整文件 Read）。

## 分组工具

过滤放在子命令**前**（`just` 薄封装不传过滤时直接调 python）。

`just` 的 `n` 为**位置参数**（`just obs-recent 40`），不要写 `n=40`。
仅 summary / recent / tui-lag 有 just 薄封装；其余子命令直接调 python：

```bash
just obs-summary
just obs-recent 40

python3 scripts/inspect_provider_trace.py requests -n 8
python3 scripts/inspect_provider_trace.py turns -n 8
python3 scripts/inspect_provider_trace.py --request-id RID lag
python3 scripts/inspect_provider_trace.py --request-id RID --turn-id TID lifecycle
python3 scripts/inspect_provider_trace.py --request-id RID channel

python3 scripts/inspect_provider_trace.py --since 30m summary
python3 scripts/inspect_provider_trace.py --path "$LOG_DIR/provider-trace.jsonl" summary
python3 scripts/inspect_provider_trace.py --turn-id TID turns -n 8
python3 scripts/inspect_provider_trace.py --request-id RID lag
```

### 排障顺序

| 怀疑 | 顺序 |
|------|------|
| 日志开没开 / 写到哪 | 健康检查 → `summary` → `requests` |
| 工具意图晚于 args | `lag`（可加 `REQUEST_ID`） |
| react span | `turns` → `lifecycle` |
| thinking/text 通道混 | `channel` + 少量 `recent` |
| 级别日志 / TUI 埋点 | `tail -n 80 "$LOG_DIR/xylitol.log"` |
| **busy spinner 卡住不动** | `just obs-tui-lag`（见下） |

### Spinner 冻结（host 环阻塞）

复现：两 MCP 配置 → 进 TUI → 立刻发长 prompt；观察 Working 先静一下才转。

```bash
XYLITOL_DEBUG=1 cargo run --   # 或 RUST_LOG=xylitol::lag=info,xylitol=warn
# 复现后：
just obs-tui-lag
```

关注 `xylitol.log` 里 phase：

| phase | 含义 |
|-------|------|
| `host_run_await` | 已画 spinner 后仍 await `XyDriver::run`（**此间无 Tick**） |
| `run_ensure_session` / `run_load_history` / `run_build_tool_schemas` | run 启动分段 |
| `mcp_settle_*` / `rebuild_system_prompt` | MCP 合并 tools + 重筑 system prompt（可堵 Tick） |
| `host_mcp_poll_refresh` | settle 后刷 welcome 卡；若伴随 `loaded_snap_connected_servers` 大数 → 曾是 `list_all_tools` RPC（已改缓存） |

≥80ms → warn（约一帧 Loader）；≥16ms → info。

### TUI 输入延迟（滚动卡住）

终端 TUI 滚动的顺滑 = **恒定小步长直跳 + 纪律性合帧，无动画层**。coalesce 是双刃剑：主动 drain 会把连吃事件并成单帧巨跳。tmux 等复用层可能把滚轮译成方向键——SGR 与键序列必须同等支持。

| 律 | 内容 |
|---|---|
| 仪器先于药方 | 先建客观签名（延迟分布 / 帧计数 / 字节量），改前改后同秤 |
| E2E 必须覆盖真输入链 | mock 直调函数漏集成层；真 PTY 注字节才算数 |
| 先删后加 | 优先减法（删缓存 / 合并 / 抽象）；加机制前先证明删无可删 |
| 观测随功能落 | lag/metrics 钩子与逻辑同 commit |

排查 SOP：

1. **产品内**：`host_wheel_paint` 分布——高→渲染内容成本；低→进 2
2. **事件侧**：`xylitol::wheel` delta 序列——±10 以上巨值=合并过度；gap 长尾=到达节拍
3. **真链**：`tests/tui_e2e/pty.rs::pty_product_wheel_latency_signature`（`just test-tui-e2e-pty`）——首响 p50 劣化=host 外；正常=回 1/2
4. **外部**：以上全清白→终端模拟器差异（换终端 cross-check）

窄读：`XYLITOL_DEBUG=1 RUST_LOG=xylitol::lag=info,xylitol::wheel=debug` → `just obs-tui-lag`。AO 性能 lab 在 `src/app/tui/lab_ao_perf.rs`（`#[ignore]`）。

### 阶段性基线（可选）

需要「观测还活着」时跑一遍即可，勿灌全文：

1. 解析 `LOG_DIR` + 健康检查表
2. `summary`（有 `schema` / kinds / 非零 request）
3. `requests` + `turns` 各若干
4. 最近一请求：`lag` + `channel`（text-only 无 tool lag 也算正常）
5. `lifecycle`（按 turn；按 request 过滤可能为 0）

报告只留：路径来源、文件大小 / mtime 是否合理、各 probe 绿 / 黄 / 红一句。

## 人类可选（agent 仍优先脚本）

- DuckDB / `lnav`：对 **`$LOG_DIR/provider-trace.jsonl`** 即席查；勿把大结果贴回 agent。
- 「漂亮时间线」→ 可选 OTLP/HTTP → Langfuse（配置 `[otel]`，默认 none）；当前默认产物仍是**专用 JSONL**，不是 OTLP。

## agent 约定

- 先跑**一个**子命令，stdout 原样用于推理，再决定下一步。
- `lag` / `lifecycle` / 健康检查已够则**停**，不要 `recent -n 500`。
- 用户要 Jaeger / Langfuse → 指向 `[otel]` + `docs/architecture/进程内观测.md`；未配置时短期用本文件读 JSONL。
