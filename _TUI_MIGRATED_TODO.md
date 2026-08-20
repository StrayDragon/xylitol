# 产品 TUI attach 迁移清单（临时）

STATUS: incomplete

> **放仓库根就是为了不被忘掉。** 禁止挪进 `llmanspec/` / `docs/`。
> 根 `AGENTS.md` 标记 `TUI_MIGRATED_TODO_REQUIRED` + `just qa`（`scripts/check_tui_migrated_todo.py`）：
> **P0 / P1 仍有未勾项时，本文件与该标记 MUST 同时存在，不得只删一边。**
> 收口：P0+P1 全勾 → **同一提交**删除本文件、AGENTS 该段、以及该 check 脚本。P2 不挡收口。
> 架构文只写已落地事实；目标语义落地后再改 `docs/architecture/`。
> c2315 一条命令 **MUST NOT** 在本清单 P0 未勾完前 apply。

状态对照：**2026-08-21** 现码（产品 TUI = attach `XyRemoteDriver` → 本机 Host `http://127.0.0.1:18790`；print / 库嵌入仍同进程）。

---

## 0. 新 TUI / Host 模式（已拍板）

**不做 NextTurn chrome，也不做「同一 run 里下一跳 LLM 换模」。**

| | 旧同进程 TUI（仍在 InProcess / ReAct） | 产品 TUI attach（目标） |
|---|---|---|
| 谁记住「选中」 | Host `ModelManager` | **Client 当前选择**；随用户提交带给 Host |
| 谁决定「这轮用哪颗」 | 每次 `prepare_turn_binding` 读 *当时* selected（工具环内可换） | **本次 `prompt` / run 开始时冻结**；整段 run（含工具环）不变 |
| busy 时 `/model` | 写入 selected；footer=生效中；status 右侧 `Next turn: …` | 立刻改 client 选中与 footer；**当前 spinner 那一轮不换**；下一句用户提交才带新值 |
| 线 | `set_model` unary，再 `prompt{message}` | `prompt` **MUST** 带 `model_id`（+ thinking）；Host 用载荷，不靠「上次 set_model 的隐式状态」打这一枪 |
| 工作区 | 进程 cwd | **TUI 启动目录**随 unary 交给 Host；按会话隔离工具 / trust / session.cwd |

**验收（改码后）**

1. Idle 切模：footer 立刻是新 id；下一句 `prompt` JSON 含该 `model_id`；Host 日志 / 生成走该模。
2. Busy 切模：footer 立刻新 id；**当前**生成 / 工具环仍用开跑时那颗；**不**出现 `Next turn:`。
3. 下一句用户提交（idle 后再发）：带新 `model_id`，Host 新 run 用新模。
4. 不把 `/model …` 当 steer 文本（c1780 已有）。
5. TUI 在目录 A 启动、Host 在目录 B：`!pwd` / 工具 / 新会话 cwd = **A**，不是 B。

**刻意不修**：为 attach 补 `active_turn` / 下轮预告。那条路 Host↔client 双态，易漂。

**改码顺序（模型条）**：`prompt` 载荷 → Host `submit_root` 冻结绑定 → TUI 发当前选择 → 再改 `docs/architecture/运行时即时设置.md`。

---

## 1. 角色（避免再把 chrome 塞进 Host）

| | Host（操作器） | Client TUI |
|---|---|---|
| 是 | 会话 JSONL、模型生成、MCP、工具、trust 闸、队列、写者租约、abort | 键、画、TTY、选区、折叠栈、**本机**剪贴板、footer 文案 |
| 不是 | 折叠 unary、viewport、快捷键 | 私读 Host 磁盘 JSONL；tick 上 `block_on` HTTP |

---

## 2. 差异总表（现码 vs 旧同进程产品面）

图例：`ok` 已对拍或刚修过 · `gap` 目标未落地 · `cs` 应留在 client · `wont` 产品 TUI 明确不做旧行为

### A. 会话 / 进出

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| A1 | 冷 `--session` / 切会话 transcript | **部分** | Host `last_seq=0` 仍回放（BDD）；**client 丢弃冷订 Agent 磁带** | JSONL 一次投影仍走 c2307 |
| A2 | 回合中途断线续订 `last_seq` | ok c2306 | 保持 | 与 A1 分岔；`last_seq>0` 不丢磁带 |
| A3 | mux 常驻、不因 `AgentEnd` 拆 WS | ok c2306 | 保持 | |
| A4 | 无 `--session` 铸造 session id | ok | 保持 | |
| A5 | 默认模型（footer 非 `NOT-SET`） | ok（writer `default_model_id`） | 保持 | 须 `xylitol serve` 与 TUI 同配置 |
| A6 | editor ↑/↓ 跨会话种子 | 弱：Remote `session_store()=None`，走 async `list_sessions` | 对拍旧面：种子仍要能出 | 勿把 store 句柄穿到 client |
| A7 | TUI pwd = 工作区；Host 多 workspace | **ok（切片）** | unary 带 `cwd`；writer 按会话用该目录 | 全局 `serve` cwd 只作缺省；MCP 进程配置仍全局 |

### B. 模型 / 提交

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| B1 | `/model` 列表不卡 tick | ok（缓存 + `refresh_surface_caches`） | 保持 | |
| B2 | `prompt` 携带当前 model/thinking | **ok** Remote 发 `model_id` + `thinking_level` | Host 开跑前 select | |
| B3 | Host 整段 run 冻结开跑模型 | **ok（Host 推迟 set_model）** | 忙时 `/model` 进 pending，AgentEnd 后才写 ModelManager | 不改 ar25 ReAct 重读；attach 不 NextTurn |
| B4 | NextTurn chrome | **wont** attach 已像「立刻切」且无预告 | 维持 **不做** | 旧文档仍写 NextTurn，改 B2/B3 后再改文 |
| B5 | Assembling / 工具表冻结 | ok（arm + poll settle） | 保持 | MCP 真连失败仍可 0 connected，看 `/mcp` |
| B6 | steer / follow_up / clear_queue | **gap** 同步 `block_on` HTTP | 改 async，与 `/model` 同纪律 | 忙时第二条消息会再卡 |

### C. MCP / 资源头

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| C1 | 头卡 connected 数跟 writer | ok（按 session 槽 poll） | 保持 | |
| C2 | 连接中 `connecting n/n` | 视 MCP 是否 Running | 失败要有 diag，不要假绿「好了」 | 0 connected + 无 diag = 仍要查 |
| C3 | `/reload` | unary 有 | 对拍取消、进度 | Remote `reload` 仍不接 TUI cancel token |
| C4 | `$skill` 补全 | **ok** 从 `loaded_resources.skill_names` 填缓存 | 描述仍空，仅 name | |
| C5 | MCP 闸超时滚动提示 | **ok** snapshot `mcp_gate_notice` + Remote take | | |

### D. 本机面（CS：不要做成 Host unary）

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| D1 | 复制 / Ctrl+V 文本 / 剪贴板图 | **gap** Remote 走 trait 默认 `unsupported` | **client 本机**实现（OSC52 + 平台工具） | 禁止 `copy_text` 进 Host |
| D2 | `/trust` persist | **gap** unsupported | Host 可有 persist；**client 调 unary**，不是 TUI 写 Host 家目录 | 与 D1 相反：trust 是 Host 事实 |
| D3 | 折叠 / 视口 | ok TUI 本地 | 保持；**不进 Host** | c2325 已 delay |
| D4 | `/export` 路径 | 半截：有 `content` 才写 **TUI 机器**；否则返回 Host 路径 | 导出文件落 **发起 export 的那台**（TUI） | Host 只出字节或写自己盘要在 UI 写清 |
| D5 | `/import` | client 读本地文件再 unary `content` | 保持 | |

### E. 生成体验 / 其它

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| E1 | footer token / 上下文估计 | Remote **本地** `estimate_from_session_entries` | 对拍是否够用；不够再 Host 结算事件 | 已有 `ContextTokenSettlement` |
| E2 | bang `!` | `execute_bash` unary | 对拍流式 chunk（现码注释：REST 无 live uplink） | |
| E3 | 内置 `ask` | reverse-RPC 已接线 | 手测一轮 | |
| E4 | abort Esc | unary `abort` spawn | 对拍 latch 文案 | |
| E5 | 主题 `/theme` | TUI 本地 | 保持 Immediate | |

---

## 3. 迁移 checklist（按顺序勾）

改码前先读本表。**禁止**为未开闸 Web 加 Host 方法。

### P0 — 产品 TUI 能当默认面用

- [x] **B2** `prompt`（及如需要的 `steer`/`follow_up`）载荷含 `model_id` + thinking；TUI 用 **client 当前选择** 填，不靠 Host footer 反推
- [x] **B3** Host `prompt` 开跑前套用载荷模型；忙时 `set_model`/`thinking` 推迟到本 run 结束（不改 ReAct ar25 重读）
- [x] **A7** TUI 启动目录随 unary 到 Host；writer 工具 / session.create / trust 用该 cwd（缺省才回落 `serve` cwd）
- [x] **A7 补**：bang `!` 落在 serve cwd 而非会话工作区。真根因：bash 执行器无 cwd 概念——`BashExecOpts.cwd` + `InfraBashExecutor.current_dir` + Driver 传 `agent.cwd()` 已修（回归：`bash_unary_runs_in_client_workspace` / `cwd_option_spawns_shell_in_workspace`）
- [ ] **A7 后续**：模型侧 bash 工具（`infra/tools/bash.rs`）与文件工具 `resolve_to_cwd` 仍继承 Host 进程 cwd；多工作区 attach 下 LLM 相对路径操作会落错目录。需给工具执行面穿会话工作区（XyToolCtx 或 per-driver 构造），走 propose
- [x] 手测 2026-08-21：idle 切模 footer 立刻变且下一句走新模（provider trace 证实 openai-responses→anthropic-messages）；busy 切模当前轮全程旧模、下一句才换、无 `Next turn:`；`/model` 不进 steer
- [x] 改架构文 `运行时即时设置.md`：产品 TUI/Host = **run 绑定**；删「attach 应对齐下轮预告」（2026-08-21 同步一轮对话/配置与档案/扩展与开闭/README/chrome 词表标注/roadmap M0；词表词条保留至 P1 随 ath22 spec 一并退役）
- [x] **A1 磁带** client 冷订丢弃 Agent 实况（c2307 快照投影仍未做）
- [x] **C4** `$skill` 从 `loaded_resources.skill_names` 缓存
- [x] **C5** `mcp_gate_notice` 进 snapshot，Remote `take_mcp_gate_notice`
- [ ] **A1 快照** c2307：`get_messages` 一次投影
- [x] **D1** 剪贴板改走 TUI 本机，去掉对 Remote `unsupported` 的依赖（`XyRemoteDriver` 活在 TUI 进程，直接镜像 InProcess 的 `infra::clipboard` 三实现；回归 `clipboard_ops_stay_client_local` 断言零 unary）
- [x] **B6** steer/follow_up/clear_queue 去 `block_on`（trait 改 async，与 `/model` 同纪律走 effects 泵；删除 Remote `block_on` helper）
- [x] **D2** `/trust` 经 Host unary（产品保留该 slash；`persist_trust` 路由会话槽 writer，按会话工作区解析信任；trait 方法改 async 与 B6 同纪律）
- [x] **D4** export 落点与文案（Host 暂存临时文件→响应带 `content` 字节→TUI 写本机盘并清理暂存；`/export` 无参时落 TUI cwd，文案显示本机路径）

### P1 — 对拍收口后才动

- [ ] **C3** reload 合作取消
- [ ] **E1/E2** token 与 bang 流式是否要 Host 事件
- [ ] `XyRemoteDriver` 拆文件（现 ~1700 行）
- [ ] 产品路径死 InProcess TUI 入口清掉（print/embed 保留）
- [ ] AGENTS / architecture 与代码同句（attach 是现行拓扑）

### P2 — 明确后置（不要提前做；不挡删除本文件）

- [ ] c2315 一条命令 loopback（仍真 `HttpWsClient`）
- [ ] c2310 TS client / c2320 oapi / c2305 ACP
- [ ] c2325 跨面动作表（TUI 已有折叠；不进 Host）

---

## 工作区（A7）

**目标**：全局 Host + 每个 TUI 用**自己的启动 pwd** 当工作区；Host 按会话并行管不同 workspace。

| | 现码（改前） | 目标 |
|---|---|---|
| Host 缺省 cwd | `xylitol serve` 进程 `current_dir` | 仅作 **缺省** |
| TUI footer | TUI 进程 `current_dir`（只画） | 与 Host 该会话工具 cwd **一致** |
| 会话列表 cwd 过滤 | 创建时写入的 cwd | 写入 **TUI 传来的 cwd** |
| MCP / skills | 一份挂 Host 进程 | skills/trust **按会话 cwd**；进程级 MCP 配置仍可全局 |

---

## 4. 每次手测最短路径（防回归）

1. 自己起 `xylitol serve` 再起 TUI（不要混用别人的 18790）。
2. 头卡：configured 与 `/mcp` 一致；连不上要有失败行。
3. 发一句：能离开 Assembling。
4. `/model`：不卡；footer 是选中 id。
5. 生成中再 `/model`：footer 变；**本轮**模型不变；打完再发才换。
6. `--session` 长历史：一次出全文、无假 spinner（A1 落地后）。
7. 复制一段 / 粘贴：在 **TUI 这台机器** 的剪贴板（D1 落地后）。
8. Host 与 TUI 不同目录启动：`!pwd` 是 TUI 目录。

---

## 5. 相关票

| 票 | 关系 |
|---|---|
| c2306 已归档 | MCP 头、队列条、tick 禁同步 RPC、mux |
| c2307 草案 | 仅 A1 快照投影 |
| c2325 delayed | 折叠动作 client-only |
| c2315+ | P2 |
