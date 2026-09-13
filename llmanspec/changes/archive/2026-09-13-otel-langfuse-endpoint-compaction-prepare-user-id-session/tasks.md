# Tasks — OTEL / Langfuse 采集盲区

## 测试 seam（复用既有 harness，不新发明）

- otel.rs 纯函数诊断 reason 单测。
- obs.rs `SpanCollectScope` span 形状单测（capability 惯例：MUST NOT 为静态存在性扩 BDD step）。
- loaded_resources.rs 渲染单测（既有风格）。

## Tasks

### T1 specs landing：otel3 扩句 + atc18 扩句 + 新 otel27

- [x] `llmanspec/specs/infra-otel/infra-otel.feature`：otel3 追加 obs_diag 可见性句；
  文件尾追加 `@req:otel27` 场景 otel-compaction-skipped-span。
- [x] `llmanspec/specs/app-tui-fixed-zone/app-tui-fixed-zone.feature`：atc18 追加 obs 行句。
- [x] `llman sdd validate <id> --strict`。

### T2 otel.rs 未生效诊断捕获

- `infra/observability/otel.rs`：纯函数 `otlp_disabled_diag_message`（exporter=none → None；
  resolve 失败 → 缺 endpoint/env 文案；build 失败 → 构建失败文案）+ `OnceLock` 进程槽 +
  `pub fn otlp_disabled_diag() -> Option<String>`；install 路径两处设置。
- 单测：三形态 reason + accessor。

### T3 snapshot 字段 + 卡渲染

- `blocked-by: T2`
- `driver/types.rs`：`LoadedResourcesSnapshot.obs_diag: Option<String>`（serde 缺省）。
- `in_process/mcp.rs`：snapshot 组装两处读诊断槽。
- `loaded_resources.rs`：obs_diag Some → `obs:` 行（warning 色，沿用 field_lines 换行）；
  None 不占行。渲染单测两形态。

### T4 compaction 早退 span

- `blocked-by: T1`
- `compaction/obs.rs`：`export_skipped(reason_detail, parent, obs)`（token.estimate 模板；
  会话身份无 lane；闸关直接返回）。
- `orchestrator.rs`：`run_auto_compaction` prepare 早退（`Ok(false)` 前）与 `compact`
  manual prepare 早退两处发射；`NoActiveSession` 不经过这两处（无会话身份不发射）。
- 单测：SpanCollectScope 下 skipped span 名/skip_reason/session id/无 xylitol.obs.lane；
  闸关无 span。

### T5 门禁与收口

- `blocked-by: T2, T3, T4`
- `just fmt` / `just lint` / `cargo test --lib`；`llman sdd validate --all --strict`。
