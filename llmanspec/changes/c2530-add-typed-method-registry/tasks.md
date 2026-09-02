# Tasks：c2530 Typed Method Registry

> 迁移策略为大范围重构的 expand-contract（design D4），不强拆垂直切片。
> 验收基线：每批合并后既有 BDD（`cargo test --test bdd`）+ 既有单测全绿；wire 行为零变化。
> 测试 seam：复用既有 harness（BDD features + host.rs/http.rs 既有单测），不新增 seam、不新增 `.feature`。

## expand

- [ ] t1 注册表骨架：`registry!` 声明宏模块 + 运行时静态方法表；与 `UNARY_METHODS` 双表并存并加「两表一致」单测；`resp`/`auth`/`idem` 能力位枚举就位（`resp: job` 仅枚举占位）
- [ ] t2 首个方法垂直验证：`set_thinking_level` 走完「声明 → 派生请求解析（serde 校验替换手抠）→ handler → 客户端类型化调用」全链，作为形态样例 `[blocked-by: t1]`

## migrate（分批，每批独立可验证）

- [ ] t3 无载荷只读批：get_state / cycle_model / get_available_models / get_commands / get_messages / get_session_stats / list_sessions / new_session / get_session_name / loaded_resources / queue_stats / session_tree 等 `[blocked-by: t2]`
- [ ] t4 带载荷读写批：set_model / set_session_name / set_session_name_for / delete_session / switch_session / fork / travel_session_tree / append_entry_label / load_session_entries / import_jsonl / export_html / export_jsonl / load_debug_scene / compact / steer / follow_up / clear_queue / bash（resp 声明为 result，作业化另行立项） `[blocked-by: t2]`
- [ ] t5 特例批：host.describe / subscribe（cwd 扩展字段）/ prompt（stream：ack + spawn）/ reload / persist_trust / arm_tool_freeze / abort（`idem: bypass` 声明化，删除行位依赖） `[blocked-by: t4]`
- [ ] t6 客户端收口：`driver/remote.rs` 全量切类型化调用方法，删除手搓 `json!` 载荷 `[blocked-by: t5]`

## contract

- [ ] t7 删除 host.rs 手搓 method→Command 转换表与 `UNARY_METHODS` 字符串表；方法表/404 语义/salvo 路由/OpenAPI 条目全部改为注册表派生；双表一致性单测随之退役 `[blocked-by: t6]`
- [ ] t8 收尾对拍：全量 BDD + 单测 + `just lint`；确认 wire 信封/幂等/事件闭集无 diff（对照 t1 前基线） `[blocked-by: t7]`
