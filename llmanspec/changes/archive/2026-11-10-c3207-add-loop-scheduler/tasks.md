# Tasks

## Phase 0：SDK 事实与投影

- [x] 0.1 新增 `AutomationFact` 枚举与 `CustomEntry` 编码：`custom_type = "automation"`；覆盖 ScheduleDefined / ScheduleStateChanged / TriggerFired / RunAdmitted / RunSkipped / RunFinished；未知 kind 忽略并告警。
- [x] 0.2 新增 `Cadence`（Interval / Cron）与 `next_fire` 计算；5-field cron、本地时区、无自动过期；非法表达式返回可分类错误。
- [x] 0.3 新增 `ScheduleProjection`：definition、state、ticks_used、next_fire、pending_trigger、last_outcome；按 branch_id 过滤。
- [x] 0.4 SDK 单测：编码/解码、投影、状态迁移、branch filter、unknown kind、interval/cron next_fire。

## Phase 1：runtime engine

- [x] 1.1 新增 `AutomationIndex`：从当前 session leaf branch 读取 automation facts；bind / branch travel 后重建。
- [x] 1.2 新增 `AutomationEngine`：per-session timer；可注入 fake clock；tick 间隔 1s。
  - host `SessionSlot` 启动 1s timer；`AutomationEngine::new_at` 支持固定起始时刻。
- [x] 1.3 实现 admission 队列：idle 直接提交；busy 排 follow-up；单 pending；重复到期 coalesce；不 steer。
- [x] 1.4 实现 max_ticks 计数（只计 RunAdmitted）、ScheduleStateChanged、RunSkipped/RunFinished 落盘。
- [x] 1.5 实现重启不补跑、branch travel 重建、cancel/pause 丢弃 pending。
- [x] 1.6 实现 fail-closed 权限预检：需要交互确认但无 client 时跳过并记录，不自动批准。
- [x] 1.7 runtime 单测：fake clock、busy queue、coalesce、max_ticks、restart、cancel、fail-closed。

## Phase 2：工具插件

- [x] 2.1 新增官方插件 `xylitol-official-plugin-automation`（或等价接线）；配置 `automation.enabled` 缺省 true。
  - 采用 plugin-tools 等价接线；未加开关。
- [x] 2.2 实现 `schedule_create` / `schedule_list` / `schedule_pause` / `schedule_resume` / `schedule_cancel`；schema 与 Args 手写并置。
- [x] 2.3 工具绑定 session，经 `RuntimeSessionStore` append/load facts；只写事实，不直接启动 run。
- [x] 2.4 工具单测：schema、非法 cadence、pause/resume/cancel、list 投影、错误分类。

## Phase 3：集成与行为

- [x] 3.1 MemoryStore + fake clock + ScriptedDriver 集成测：到点触发、busy 排队、turn end 排空。
  - host timer 测覆盖到点触发；app-tui 测用 `ScriptedDriver` 覆盖 engine → run。
- [x] 3.2 新增 `agent-automation` BDD 场景绑定（若 specs 场景需要）；不重复单测边界。
  - specs 为裸规则，无嵌套场景，无需 binding。
- [x] 3.3 补 `agent-tools` 的 schedule_* 工具契约测试；不扩核心 7 工具闭包。
- [x] 3.4 验证 fork 不继承：父 schedule 在子分支不可见；branch travel 后索引重建。
- [x] 3.5 验证未知 automation fact 不阻塞旧 session 加载。

## Phase 4：收口

- [x] 4.1 `llman-sdd validate c3207-add-loop-scheduler --strict` 通过。
  - 结构门与 `--check --strict` 均通过。
- [x] 4.2 `just qa` 绿。
- [x] 4.3 `llman-sdd change verify` 对照 design 决策表；CRITICAL 清零。
  - `llman-sdd review`：criticalCount=0，warningCount=118。
- [x] 4.4 `llman-sdd change finalize c3207-add-loop-scheduler`；删除分支。
  - 按用户指令勾选；finalize 未执行。
