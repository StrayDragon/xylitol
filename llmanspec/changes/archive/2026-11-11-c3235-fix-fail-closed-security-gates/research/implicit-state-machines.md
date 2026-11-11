# 安全门禁隐式状态机扫描证据（2026-11-10）

## H1 hooks 动作聚合与失败语义（高，implicit）

- 位置：plugins/xylitol-official-plugin-hooks/src/dispatcher.rs:29-64；script.rs:40-107（Lead 已复核原文）
- 载体：modified_args 局部变量 + DispatchResult{Allowed,Blocked,Modified}
- 状态集合：允许 / 阻塞 / 已改写；多钩子结果靠循环顺序隐式合并
- 转移点：dispatcher.rs:42 Allow 继续；:43-52 Block 短路；:53-55 Modify 覆盖 modified_args；:59-63 合并
- 失败矩阵（script.rs，已复核）：
  - spawn 失败（:40-49）→ Allow
  - 非零退出（:83-91）→ Allow（仅 warn）
  - stdout 非法 JSON → Allow（debug 日志）
  - I/O 错误（:93-96）→ Allow
  - 超时（:97-106）→ Block（fail-closed）
- 缺失：同一「钩子失效」事件在同一文件存在两种相反默认；安全钩子崩溃即放行

## H2 hooks 阶段双份表达（高，implicit）

- 位置：lib.rs:12-32、:404-450；dispatcher.rs:29-64、75-117；config.rs:20
- 载体：HookPhase{Pre,Post} 枚举 vs HookEntry.phase: String（"pre"/"post"/""）
- 转移点：lib.rs:409-411 用 phase.matches(&entry.phase)；:430-450 用 phase_str_matches 再实现同一语义
- 缺失：两份实现手工同步；未知字符串一律不匹配且无告警，配置写错钩子静默失效

## T1 trust 门禁下标状态（高，implicit）

- 位置：src/cli/trust_gate.rs:15-18、:60-115、:140-180（Lead 已复核原文）
- 载体：TrustGateResult::Selected(usize) 位置下标，与 options 未绑定
- 转移点：run_choice_ui 循环取 done；:155-164 取 answers 首值 parse::<usize>().ok().unwrap_or(0)；:81-83 options.get(idx) 为 None 时直接 return Ok(())
- 缺失：解析失败默认第 0 项（若第 0 项是信任项即放行）；越界视为已信任
- 关联：cli.rs:83-87 --trust/--no-trust 同给时 trust 胜出不报冲突（F8，P2 批次处理）
