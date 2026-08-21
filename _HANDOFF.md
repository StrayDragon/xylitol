# 产品 TUI attach：对拍总表（交接）

> **用途**：换机 / 换会话继续 attach 对拍时用。真值源仍是代码 + `_TUI_MIGRATED_TODO.md`（qa 闸）；本文件是整理后的可读总表，**不替代** checklist 闸。
>
> **快照日期**：2026-08-21 · **分支**：`main`（当时 ahead of origin 6 commits，含补全高亮 + c2330 草案）

---

## 0. 给 Agent / 协作者

| 规则 | 说明 |
|---|---|
| **Host 谁起** | **人类**起 `xylitol serve`（默认 `127.0.0.1:18790`），保证全局只有一个 listener。Agent **除临时测试验证外 MUST NOT 主动启动 server**。 |
| **TUI** | 产品 TUI attach 该 Host；print / 库嵌入仍可同进程。 |
| **Worktree** | 并行树先 `eval "$(just cargo-wt-env)"`，勿共用 `CARGO_TARGET_DIR`。 |
| **c2315** | P0 未勾完 **禁止 apply** `c2315-add-loopback-host-tui`。 |
| **live specs** | 默认分支 **禁止** 为已落地实现直接改 live specs；合约缺口走 SDD（例：`c2330-add-session-resources-downlink` 草案）。 |
| **收口** | P0+P1 全勾 → **同一提交**删 `_TUI_MIGRATED_TODO.md`、根 `AGENTS.md` 的 `TUI_MIGRATED_TODO_REQUIRED` 段、`scripts/check_tui_migrated_todo.py`。P2 不挡。 |

**拓扑（现行）**

```
产品 TUI (client) ──HTTP unary + WS mux──► Host (操作器) @ 127.0.0.1:18790
     │                                         │
     键/画/TTY/补全/折叠/剪贴板(待)              JSONL / ReAct / MCP / trust / 队列
```

---

## 1. 已拍板产品语义（attach ≠ 旧同进程 NextTurn）

**不做** NextTurn chrome；**不做**同一 run 内工具环中途换模。

| | 旧同进程 TUI | 产品 TUI attach |
|---|---|---|
| 谁记住「选中」 | Host `ModelManager` | **Client 当前选择**；随用户提交带给 Host |
| 谁决定「这轮用哪颗」 | 每次 `prepare_turn_binding` 读当时 selected | **`prompt` / run 开始时冻结**；整段 run（含工具环）不变 |
| busy 时 `/model` | footer + `Next turn: …` | footer **立刻**变；**当前 spinner 不换**；下一句提交才带新 `model_id` |
| 线 | `set_model` 再 `prompt` | `prompt` **MUST** 带 `model_id` + thinking；Host 用载荷 |
| 工作区 | 进程 cwd | **TUI 启动 pwd** 随 unary 给 Host；会话工具 / trust / session.cwd 按此隔离 |

**验收要点**

1. Idle 切模 → footer 新 id → 下一句 `prompt` JSON 含该 `model_id`。
2. Busy 切模 → footer 新 id → **本轮**生成仍用开跑时那颗 → **无** `Next turn:`。
3. 下一句 idle 提交 → 新 run 用新模。
4. `/model …` 不当 steer 文本。
5. TUI 在 A、Host 在 B → `!pwd` / 工具 cwd = **A**。

**刻意不修**：attach 补 `active_turn` / 下轮预告（双态易漂）。

---

## 2. 角色（别把 chrome 塞进 Host）

| | Host（操作器） | Client TUI |
|---|---|---|
| **是** | 会话 JSONL、生成、MCP、工具、trust 闸、队列、写者租约、abort | 键、画、TTY、选区、折叠栈、**本机**剪贴板、footer |
| **不是** | 折叠 unary、viewport、快捷键 | 私读 Host 磁盘 JSONL；tick 上 `block_on` HTTP |

---

## 3. 差异总表（现码 vs 旧同进程产品面）

图例：`ok` 已对拍 · `gap` 未落地 · `cs` 应留 client · `wont` 明确不做

### A. 会话 / 进出

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| A1 | 冷 `--session` / 切会话 transcript | **部分** | client 冷订丢弃 Agent 磁带；快照一次投影 | 快照仍走 **c2307** |
| A2 | 回合中途断线 `last_seq` 续订 | ok c2306 | 保持 | 与 A1 分岔 |
| A3 | mux 常驻、不因 `AgentEnd` 拆 WS | ok c2306 | 保持 | |
| A4 | 无 `--session` 铸造 id | ok | 保持 | |
| A5 | 默认模型 footer | ok | 保持 | serve 与 TUI 同配置 |
| A6 | editor ↑/↓ 跨会话种子 | 弱 | 对拍旧面 | Remote 无 `session_store()` |
| A7 | TUI pwd = 工作区 | **ok** | unary 带 `cwd` | serve cwd 仅缺省 |

### B. 模型 / 提交

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| B1 | `/model` 列表不卡 tick | ok | 保持 | 缓存 + `refresh_surface_caches` |
| B2 | `prompt` 带 model/thinking | **ok** | Host 开跑前 select | |
| B3 | Host run 冻结开跑模型 | **ok** | 忙时推迟 set_model | attach 无 NextTurn |
| B4 | NextTurn chrome | **wont** | 维持不做 | 文档待改 |
| B5 | Assembling / 工具表冻结 | ok | 保持 | arm + poll settle |
| B6 | steer / follow_up / clear_queue | **gap** | 去 `block_on` | 忙时第二条会卡 |

### C. MCP / 资源头

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| C1 | 头卡 connected 数 | ok | 保持 | session 槽 poll |
| C2 | connecting n/n / 失败 diag | 视 MCP | 失败要有 diag | 勿假绿 |
| C3 | `/reload` | unary 有 | 对拍取消 | Remote 未接 cancel token |
| C4 | `$skill` 补全 | **ok** | 保持 | 从 skill_names 缓存 |
| C5 | MCP 闸超时滚动提示 | **ok** | 保持 | snapshot + Remote take |

**B（下行）**：MCP chrome 已走 mux `session/resources`（不进 journal）；TUI tick 只读 dirty/cache，不 16Hz unary `loaded_resources`。合约草案：**c2330**（未 propose）。

### D. 本机面（client-side）

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| D1 | 复制 / 粘贴 / 剪贴板图 | **gap** | TUI 本机 OSC52 等 | 禁止 Host `copy_text` |
| D2 | `/trust` persist | **gap** | client 调 Host unary | trust 是 Host 事实 |
| D3 | 折叠 / 视口 | ok | 保持 client | c2325 delay |
| D4 | `/export` 路径 | 半截 | 文件落 **TUI 机器** | 文案要说清 |
| D5 | `/import` | ok | 保持 | client 读盘再 unary |

### E. 生成体验 / 其它

| id | 能力 | 现码 | 目标 | 备注 |
|---|---|---|---|---|
| E1 | footer token 估计 | Remote 本地 estimate | 对拍够用否 | 已有 Settlement 事件 |
| E2 | bang `!` | execute_bash unary | 流式 chunk | 无 live uplink |
| E3 | 内置 `ask` | reverse-RPC | 手测 | |
| E4 | abort Esc | unary abort | 对拍 latch | |
| E5 | `/theme` | TUI 本地 | 保持 | |

---

## 4. 迁移 checklist（执行顺序）

### P0 — 产品 TUI 能当默认面

- [x] B2 prompt 载荷 model/thinking
- [x] B3 Host run 冻结 + 忙时推迟 set_model
- [ ] 手测 idle/busy 切模、无 `Next turn:`
- [ ] 改 `docs/architecture/运行时即时设置.md`（run 绑定，删 NextTurn attach 叙述）
- [x] A1 磁带 client 冷订丢弃 Agent 实况
- [ ] **A1 快照** c2307：`get_messages` 一次投影
- [x] A7 TUI cwd 随 unary
- [x] C4 / C5
- [ ] D1 剪贴板本机
- [ ] B6 steer/follow_up/clear_queue async
- [ ] D2 `/trust` unary
- [ ] D4 export 落点文案

### P1 — 对拍收口后

- [ ] C3 reload 合作取消
- [ ] E1/E2 token / bang 流式
- [ ] 拆 `XyRemoteDriver`（~1700 行）
- [ ] 清死 InProcess 产品 TUI 入口
- [ ] AGENTS / architecture 与 attach 拓扑同句

### P2 — 后置（不挡删 TODO）

- [ ] c2315 loopback 一条命令
- [ ] c2310 TS / c2320 oapi / c2305 ACP
- [ ] c2325 跨面动作（TUI 折叠 client-only）

---

## 5. 本会话已落地（换机后不必重做）

| 主题 | commit / 位置 | 要点 |
|---|---|---|
| follow-up/steer 命令栏闪退 | `20504afc` | `set_completion_sources` 再探测不关 popup；busy Enter 先 confirm 补全 |
| MCP chrome 下行 | `fab4f2f7` | Host poll → mux `session/resources`；Remote dirty 标志 |
| 补全高亮 Tab/Enter | `507fc3a5` | prefix 不变时保 highlighted `value`（全 CompletionSource，非仅 `/model`） |
| attach 清单闸 | `7bf46192` | `_TUI_MIGRATED_TODO.md` + check 脚本 |
| SDD 草案 c2330 | `6f091e20` | `llmanspec/changes/c2330-add-session-resources-downlink/` |

**手测已通过（当时）**：follow-up 下 `/mode` 命令栏；MCP B 下行；补全高亮多候选 Tab/Enter。

---

## 6. 手测最短路径

1. **你**起 `xylitol serve`，再起 TUI（勿混用他人 18790）。
2. 头卡 configured 与 `/mcp` 一致；失败有 diag。
3. 发一句 → 离开 Assembling。
4. `/model` 不卡；footer = 选中 id。
5. 生成中 `/model`：footer 变、本轮不换、下一句才换。
6. follow-up 条存在时 `/mode` 或 `/model dee`：命令栏稳定；↓ 高亮 + Tab/Enter 应用**高亮行**。
7. `--session` 长历史：一次全文、无假 spinner（**待 A1 快照**）。
8. 复制粘贴（**待 D1**）：TUI 本机剪贴板。
9. Host/TUI 不同目录：`!pwd` = TUI 目录。

---

## 7. 相关 SDD / 文档

| 路径 | 说明 |
|---|---|
| `_TUI_MIGRATED_TODO.md` | qa 闸 checklist（P0/P1 未勾完不可删） |
| `llmanspec/changes/c2307-fix-tui-attach-cold-restore/` | A1 快照投影草案 |
| `llmanspec/changes/c2330-add-session-resources-downlink/` | `session/resources` 进 server-core 合约草案 |
| `llmanspec/changes/c2325-add-cross-surface-actions/` | client-only 折叠动作（delay） |
| `llmanspec/changes/c2315-add-loopback-host-tui/` | P2，一条命令 |
| `docs/architecture/运行时即时设置.md` | **待改**（仍写 NextTurn 旧语义） |
| `src/app/tui/AGENTS.md` | 产品 TUI 硬约束 |

---

## 8. 建议下一刀（P0 优先）

1. **c2307 propose → apply**：冷 attach `get_messages` 一次投影，与磁带丢弃分岔写清。
2. **B6**：steer / follow_up / clear_queue 改 async，对齐 `/model` 不 block tick。
3. **D1**：Remote 剪贴板 trait → TUI 本机（OSC52 / wl-copy / xclip 等）。
4. 手测勾 P0 两项 + 改 `运行时即时设置.md`。
5. 准备 propose **c2330**（若要把 `session/resources` 写进 `server-core` w1）。

---

## 9. 开发机起手

```bash
git pull   # 或 fetch 含上述 commit 的分支
eval "$(just cargo-wt-env)"
just setup
# 终端 1（人类）：
xylitol serve --host 127.0.0.1 --port 18790
# 终端 2：
cargo run --   # 或你们惯用的 TUI 入口
just test-tui  # 改 TUI 后
just qa        # 开 PR 前
```
