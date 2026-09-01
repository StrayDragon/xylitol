# Design：c2535 Staged Wire Import

## D1 对称暂存（对齐 staged export）

`export_html/export_jsonl` 的既有暂存语义（c2530 前落地）：无 `output_path` 时 Host 写唯一临时文件、dispatch 后读回 `content` 并删除——「文件落在请求它的那台机器上」。import 为同一语义反向：客户端推送 `content`，Host 暂存为 `xylitol-import-{uuid}.jsonl`、以 `input_path` dispatch、结束后删除。两块暂存并存于 writer 分支，互不影响。

## D2 响应键已对齐，零响应侧改动

`dispatch(ImportJsonl)` 返回 `DispatchOutcome::NewSession(new_id)`，`outcome_to_value` 映射为 `{"session_id": id}`——与 `RemoteDriver::import_jsonl` 读取 `data["session_id"]` 的既有期待一致。本 change 不触碰响应形状。

## D3 幂等与租约交互不变

暂存发生在幂等准入（c2460）与租约准入（c2465 前的 WriterLease）**之后**、parse 之前：重试同一 rpcId 时幂等层回放首个结果，暂存块不会二次执行；`auth: writer` 位不变。临时文件清理覆盖 dispatch 成功与失败两条出路。

## D4 明确不做

- 客户端改动、分块/流式传输优化；
- `input_path` 直传语义的废弃（同机直传仍被接受，别名 `path` 一并保留——c2530 已有 alias 守卫）；
- 相邻死码（`Event::Response` 等）。
