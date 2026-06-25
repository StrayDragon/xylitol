# Design: c255-consolidate-trust-and-session

涉及 `ar02` 语义变更、trust 安全决策、跨层接线(interface→agent→infra),属权衡/迁移,故出 design。

## 1. `ar02` 修订:补上限子句

### 现状(bug)
`architecture/ar02` 原文:
> "Components with fewer than 100 lines or 5 methods and no behavioral logic MUST be inlined into their parent struct (AgentSession)."

只有"下限"(小的必须内联),没有"上限"(大的必须拆)。`c240` 据此把 `ToolManager` 内联进 `session.rs`,随后 `session.rs` 无界膨胀至 1245 行,无人拦。

### 修订(对称补全)
保留原下限,追加:

> "Conversely, inlined blocks that exceed approximately 100 lines, have independent behavioral logic, and are conceptually cohesive MUST be extracted as named collaborator objects (per agent-session.as30). The AgentSession facade MUST NOT grow unboundedly as a single struct or single file."

### 判定标准(三重必要条件)
一个内联块**同时**满足以下三点时,**MUST** 提取为协作对象:
1. **体量**:超过 ~100 行(与下限阈值对称)。
2. **行为逻辑**:不只是 field 存储 + getter 委托(那类仍按原 `ar02` 内联)。
3. **概念内聚**:有清晰的单一职责边界(状态机、IO 协议、转换格式等)。

据此,`AgentSession` 的 23 个 section 中,达标的有:auto-retry(状态机,~140行)、bash-exec(协议处理,~95行)、export/import(格式转换,~90行)。steering(~65行)、prompt(~20行)、stats(~60行)**不达体量阈值**,本次按 S1 并入 `session/` 子模块但**不**提取为独立 struct(保留作为 future 候选)。

## 2. Trust 单一 SSOT

### 2.1 归属层

```
interface/  ──提供 on_prompt 回调──┐
                                   ▼
agent/  ──调用 resolve(cwd, override, policy, has_ui, on_prompt)──► infra/trust/
                                                                   │
                                                                   ▼
                                                          ~/.xylitol/trust.json
```

- **`infra/trust/`** 是 SSOT:store(读写 trust.json + 父目录继承 + 文件锁)、resolve(优先级管线)。
- **`agent/`** 不持有信任逻辑,仅作为启动时的调用点。
- **`interface/`** 提供 `on_prompt` 回调实现(终端选择 UI),`infra/trust/` 与 `agent/` 都不直接做终端 IO(t4)。

### 2.2 持久化格式(pi 风格,沿用 `infra/trust/mod.rs` 现设计)

```json
{
  "/home/user": true,
  "/home/user/projects/evil": false,
  "/home/user/projects/old": null
}
```

- key = 规范化绝对路径;value = `true`(信任)/ `false`(不信任)/ `null`(清除)。
- 查询走**父目录继承**:从 cwd 逐级上溯找最近祖先的决策。
- **为何选这个**:`infra/trust/mod.rs` 已实现且与 pi `trust-manager.ts` 语义一致;"先 pi 风格,体验后续改进"是已确认决策。
- **合并 `agent/trust/store.rs` 的锁机制**:`agent/trust/` 版有 `O_CREAT|O_EXCL` lock 文件 + 重试,比 `infra/trust/` 的原子写更稳健。本次将锁机制迁入 `infra/trust/` 的 store 实现,丢弃其余死代码。

### 2.3 解析管线(t3,固定优先级)

```
1. trust_override = Some(b) ──► (b, Override)
2. !has_trust_inputs(cwd)   ──► (true, NoTrustInputs)
3. store.get(cwd) = Some(b) ──► (b, Store)
4. default_policy:
     Always ──► (true, DefaultPolicy)
     Never  ──► (false, DefaultPolicy)
     Ask    ──► continue
5. !has_ui                 ──► (false, FallbackNoUi)
   has_ui  + on_prompt(opts) = Some(idx):
     persist + ──► (opts[idx].trusted, UserPrompt)
     None    ──► (false, FallbackNoUi)
```

`has_trust_inputs` 判定:存在 `.xylitol/` 目录,或任一祖先存在 `.agents/skills/` 目录(沿用 `agent/trust/store.rs` 的判定,迁入 `infra/trust/`)。

### 2.4 接线(t5, t6)

- **启动门禁**:agent 启动时,`interface/` 用 cwd + CLI override 调 `infra/trust::resolve()`(注入 prompt 回调),得到 `(trusted, reason)`。结果传入 `SettingsManager::from_storage(..., project_trusted=trusted)` 的现有参数(而非删除该参数),保持 API 兼容;但移除 `SettingsManager::set_project_trusted()` 这个裸切换器(改为"信任由解析决定,不可运行时随意翻转")。
- **`/trust` 命令**(t6):`commands.rs` 的 `("trust", ...)` 接线到 `infra/trust::TrustStore::set(cwd, Some(true))`,随后刷新 `SettingsManager` 的 project_trusted。`/no-trust` 对称。

## 3. AgentSession 协作对象解构(as32)

### 3.1 目标结构

```
src/agent/session/
├── mod.rs          ← AgentSession 结构体 + new() + 字段访问 + 委托方法(<600行)
├── io.rs           ← 吸并 session_io.rs,修空桩 stats()
├── retry.rs        ← AutoRetryEngine(RetryState + is_retryable_error + 重试循环)
├── bash_exec.rs    ← BashExecHandler(bash_cancel + !cmd/!!cmd 处理)
├── export.rs       ← SessionExporter(export/import,字节级保真)
├── stats.rs        ← SessionStats + ContextUsage + token 估算
├── prompt.rs       ← 动态系统提示构建 + PromptResult
└── steering.rs     ← MessageQueue steer/followUp 访问器
```

### 3.2 协作对象契约

每个协作对象遵循统一模式(承 `c240` 对 ModelManager 的处理):

| 协作对象 | 持有状态 | 行为 | AgentSession 委托 |
|---|---|---|---|
| `AutoRetryEngine` | `retry_state: Option<RetryState>` | 判断可重试性、推进状态机 | `set_retry`/`clear_retry`/`should_retry` |
| `BashExecHandler` | `bash_cancel: Option<CancellationToken>` | 解析 `!`/`!!`、执行、取消 | `run_bash`/`cancel_bash` |
| `SessionExporter` | (无状态,持有 SessionManager 引用) | HTML/JSON 导出、导入 | `export_html`/`import` |

`AgentSession` 把这三个作为字段组合,其余(model/tools/io/compaction/skills)保持现有委托不变。

### 3.3 公共 API 保真(as31 验收)

- 所有现有 `pub fn` 签名**不变**;`AgentSession` 方法体改为委托给协作对象。
- 事件流(`AgentLifecycleEvent`)**不变**——协作对象通过 `EventBus` 引用发事件,而非自己拥有总线。
- 验收:insta 快照对比重构前后 `AgentSession` 的 `cargo doc` 公共 API + BDD 全绿。

### 3.4 空桩修复

`SessionIO::stats()` 当前返回写死的 0(`// Simplified: return empty stats for now`)。本次在 `io.rs` 实现真实统计(从 SessionManager 读条目计数),否则 as32 的 "stats 模块" 是空中楼阁。

## 4. 风险与缓解

| 风险 | 缓解 |
|---|---|
| trust 门禁(t5)误拒合法项目 | 阶段1补特征化测试覆盖"已信任项目正常加载";默认策略 `Ask` + UI 回调兜底;先用临时配置开关,确认后再硬门禁 |
| 1245 行搬运漏字段 | 每搬一个子模块跑一次 `just qa` + insta 快照;协作对象逐个提取(非一次性) |
| `ar02` 修订影响既有合规判定 | 修订是**补上限**,不撤销下限;既有内联合规(`ToolManager`)不受影响 |
| BDD 因 session 重组红掉 | as31 强制公共 API 不变;BDD 走公共 API,理论上免疫;阶段3全量验证 |

## 5. 不做的事(边界)

- 不实现 `c240` 评估的宏(`#[tool]` 等)。
- 不改 `infra/session/` 持久化格式。
- 不动 provider/loop/compaction 算法。
- trust 不做"体验改进"(如更细粒度的资源级信任、UI 美化)——留 future.md。
