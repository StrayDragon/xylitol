# 三份身份（代码对照）

探索备忘，自 c2580 伞移入 c2620（供应商键策略）。产品叙述见 `docs/architecture/会话与持久化.md`。

## 今日接线

| 身份 | 今日落点 | fork 之后 |
|---|---|---|
| 书签 | JSONL 文件 id；`AgentCapabilities::set_session`；Langfuse `langfuse.session.id` | 新 `child_id`；`create(..., parent)` |
| 树干前缀 | `get_branch` 路径拷贝；`session_env` date/cwd/clock | 同日同 cwd 不因 clock 再 append；拷来的 clock 保留 |
| 供应商键 | 默认不发 `prompt_cache_key` / `previous_response_id`；OpenCode 发 `x-opencode-session`=书签 | 子本新 header |

## 观测槽

（本节竞态已由 c2590 修复：观测快照跟本次 generate，进程槽仅作未收到 options 路径的 fallback。以下为当时事实，留档。）

`packages/xylitol-ai-bridge/src/provider/obs_session.rs`：进程 `Mutex` + 测试用 TLS `ObsSessionScope`。

生产：`set_session` → `set_obs_session` 写进程槽。Host `SessionSlot` 可并行 `run_inflight`（`src/app/server/host.rs`）。后一次 bind 会污染先一次未完成的 `llm.request` 的 `langfuse.session.id` 与 OpenCode header。

单测注释已承认此竞态（`in_process/tests/mod.rs` 用 Scope 隔离）。

## 不要做的短路

- 把书签 UUID 写进模型稿（token 0 前缀炸弹）。
- 用书签当 `prompt_cache_key`。
- 子本继承父本**切点之后**的 `previous_response_id`。
- 用观测属性驱动 TUI（已有 `XyEvent`）。
