# Tasks: c1690-fix-session-jsonl-camelcase-ssot

## 1. 合约钉死 camel SSOT

- [x] 1.1 更新 `agent-session-store`：`s2` 类型列表改 camel；收紧 `s18`；新增 skip/warn≤3 与 list 韧性 req；`scenarios`/`feature` 对齐
- [x] 1.2 更新 `agent-session`：`as45` 去掉「旧顶层 bashExecution 读提升」；与 as46 skip 策略交叉引用

## 2. 写路径只出最新

- [x] 2.1 `SessionManager::create`（及 clone header）写入 `version: SESSION_VERSION`（5）
- [x] 2.2 确认 bang-bash 仅 nested `message`+`role=bashExecution`；删除/停用顶层 `append_bash_execution` 写路径（若仍存在）
- [x] 2.3 单测：新会话 JSONL 外壳 camel + version=5 + 无顶层 bash

## 3. 读路径剔除旧栈

- [x] 3.1 删除 `serde(alias = "bash_execution")`、`migrate_v3_to_v4`、`lift_bash_execution_entry`（及调用点）
- [x] 3.2 实现行级 skip + warn 计数（≤3 后 `...`）；header version 非 5 的明确失败/拒绝策略按 design
- [x] 3.3 `list_sessions`：单文件失败 soft-skip，不失败整表
- [x] 3.4 拒绝样例单测：`bash_execution` / 未知 type / 坏 JSON → skip；warn 封顶

## 4. 面与闸

- [x] 4.1 TUI resume / print session 路径走同一 SessionManager（必要时补 system/eprintln 观察 warn，不各写解析）
- [x] 4.2 跑相关 BDD + `just test` 窄闸；修正 feature 中仍写 snake 类型名的断言
- [x] 4.3 `llman sdd validate c1690-fix-session-jsonl-camelcase-ssot --strict`
