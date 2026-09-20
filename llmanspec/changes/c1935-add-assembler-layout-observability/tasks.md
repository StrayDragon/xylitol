# Tasks: c1935-add-assembler-layout-observability

> 当前仅完成 Designed / pre-start 规划。以下任务全部留待 Branch binding 后执行；
> 本阶段不执行 `start`、`attach`、Specs landing 或代码修改。
>
> 任务依赖：所有实现任务依赖 `c1890` 已归档；Specs 任务必须先于同一分支上的
> apply 实现，并遵守 BDD-on 的 Partitioned SSOT。

## 0. Scope gate

- [x] 0.1 固定本 change 的观测字段、namespace、双通道落点、cache 诚实语义与
      body-hash 后置决定；以 `design.md` §3 为准。
- [x] 0.2 确认 `c1890-add-responses-context-policy-assembler` 已归档，且本 change
      不重新实现 ContextPolicy / WirePolicy / Responses body。
- [ ] 0.3 Branch binding 后复核工作树与 `llman-sdd show` 状态；若依赖未归档或
      change id/范围发生漂移，停止 apply 并先修正文档。

## 1. Specs landing（绑定分支后）

- [ ] 1.1 在 `infra-observability` live spec 增加可观察契约：观测闸开启时，
      Responses 的 `llm.request` 产生一条 `assembler.layout` 低频事件；字段使用
      `xylitol.layout.*`；普通 chunk 不重复布局字段；关闸不阻断主路径。
- [ ] 1.2 在 `infra-otel` live spec 增加可观察契约：同一 layout decision 进入
      `llm.request` span，保持已有 parent/session/lane 语义；不因本 change 新增
      exporter 或默认 observation I/O。
- [ ] 1.3 在 `package-ai-bridge` live spec 增加可观察契约：Responses Assembler
      的 layout decision 与实际 `WirePolicy` / thinking replay / wire mode 同源；
      当前 replay 固定全量，`chained` 只作为后续实际能力的值，不在本 change 实现。
- [ ] 1.4 按 BDD-on Partitioned SSOT 将可执行 GWT 放入对应 `.feature`，用
      `@req:` 关联；不可执行的 bridge 细节只留 `spec.toon`，不得在 toon 与
      feature 双写 scenario。

## 2. Slice A — context hint 到 Assembler decision

- [ ] 2.1 增加小型 bridge-neutral layout hint seam，承载本轮
      `tools_mode`、`status_bar_mode`、`date_placement`；不得让 bridge 依赖
      `src/agent::ContextPolicy`，不得新增 YAML/env 配置面。
- [ ] 2.2 在 turn binding 处从 immutable `ContextPolicy` snapshot 生成 hint，并经
      `XyGenerateOptions` → adapter → `AiBridgeGenerateOptions` 一对一传递；不改变
      `obs_parent` 的 span nesting。
- [ ] 2.3 扩展 ResponsesAssembler 的等价 typed result/diagnostics seam，使
      layout decision 与 body 同一次组装产生：`api`、实际 `compat`、hint 字段、
      原样 `thinking_level`、`replay_mode=full`、实际 `wire_mode`。
- [ ] 2.4 单测覆盖 Generic/Deepseek、`full`/`search`、状态栏档、date placement、
      freeform thinking level；验证 `allows_previous_response_id` 单独为 true 时
      仍是 `full_replay`，不能伪造 `chained`。

## 3. Slice B — fastrace span 与本地 provider trace

- [ ] 3.1 在 `ProviderRequestTrace` 增加 layout attach/emit helper：写入
      `xylitol.layout.*` span properties，并在 body 组装后、HTTP 前追加单条
      `kind=layout,event=assembler.layout`；闸关时不生成布局事件。
- [ ] 3.2 让 stream 与 non-stream Responses 路径都调用同一 helper；确认
      `obs_parent`、request id、HTTP error 与 abort 生命周期保持现有语义。
- [ ] 3.3 在完成 usage 时追加低频 `kind=usage` 投影，保留 input/output/total 与
      cache provenance；仅 `PromptCacheRead::Tokens(n)` 写 `cache_read=n`，对
      `NotReported` / `NotApplicable` 不写 0。
- [ ] 3.4 扩展 `FileTraceReporter` 的事件 allowlist，使 layout/usage 字段落到
      `provider-trace.jsonl` 的对应单行；不得把 root layout 属性复制到每个 raw/
      mapped chunk，不得升级 v1 schema。
- [ ] 3.5 用 `ObsGateScope` + `SpanCollectScope` 覆盖：闸开/关、单条 layout、
      span properties、parent correlation、usage 三态、abort 无伪 usage。
- [ ] 3.6 用临时文件或等价 reporter seam 覆盖 JSONL 投影：字段可窄读、旧必需
      字段仍存在、reporter 写失败不传播到 provider 主路径。

## 4. Slice C — 窄读与回归证据

- [ ] 4.1 扩展既有 `inspect_provider_trace.py` / `just obs-*` 的窄读指针，让
      排障者能按 request/turn 对照 `assembler.layout` 与 `usage` 的
      `prompt_cache_read`；不创建 Inspect UI，不读取或打印完整 body。
- [ ] 4.2 更新 runtime-log skill 的一例：先看 layout，再看 cache usage；明确
      `NotReported` 不等于 cache miss，并提醒 observation I/O 与 layout event
      是两个独立通道。
- [ ] 4.3 运行 bridge、infra observability/otel 相关单测与 BDD；必要时运行
      `just test`、`just lint`、`just fmt-check`，确认观测失败不会改变 provider
      返回值。
- [ ] 4.4 若需真实网关证据，只使用既有 `lab_` / `test-live-provider` 闸门；
      不把 body、reasoning signature、cache key 或敏感 observation I/O 写入提交或
      公共报告。

## 5. Gate and handoff

- [ ] 5.1 `llman-sdd validate c1935-add-assembler-layout-observability --strict
      --no-interactive` 通过，且 `valid_scope` / feature 绑定无缺失。
- [ ] 5.2 对照 `design.md` 的决策表逐项检查：没有新配置面、第二 tracing 栈、
      chain/thinking 实现、body hash 或 UI scope creep。
- [ ] 5.3 apply 后交给 `llman-sdd-verify`；只有 verify 全绿且
      `readyToImplement` / 生命周期条件满足后，才允许 archive。

## Start readiness（当前）

- [x] 设计、Open Questions 与测试 seam 已钉。
- [x] `c1890` 依赖已归档。
- [x] 变更边界已锁定为 provider-trace / fastrace layout observability。
- [ ] Branch binding：未执行（本请求明确保持 pre-start）。
- [ ] Specs landing：未执行（禁止在默认分支执行）。
- [ ] Apply：未就绪，等待绑定分支、specs landing 与对应门禁。
