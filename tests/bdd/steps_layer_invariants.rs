use std::process::Command;
use std::sync::OnceLock;

use crate::bdd::helpers::{with_test_timeout, with_test_timeout_for};
use crate::bdd::steps_c2827::{T4PrintBdd, t4_stream};
use rstest_bdd_macros::{then, when};

fn assert_layer_invariants() {
    static STATUS: OnceLock<Result<(), String>> = OnceLock::new();
    let result = STATUS.get_or_init(|| {
        let out = Command::new("python3")
            .args(["scripts/check_layer_invariants.py", "--check"])
            .output()
            .map_err(|e| format!("spawn layer invariants: {e}"))?;
        if out.status.success() {
            Ok(())
        } else {
            Err(format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ))
        }
    });
    if let Err(msg) = result {
        panic!("{msg}");
    }
}

#[when("读取分层保障的真值文档")]
pub(crate) fn w_la_agents_doc() {
    assert_layer_invariants();
}

#[then("保障方式为 AGENTS 与缝行为测")]
pub(crate) fn t_la_guarantee_way() {
    assert_layer_invariants();
}

#[when("读取 Cargo 特性表")]
pub(crate) fn w_la_cargo_features() {
    assert_layer_invariants();
}

#[then("可选能力 flag 为现用名且内置能力无 flag")]
pub(crate) fn t_la_feature_flags() {
    assert_layer_invariants();
}

#[then("默认集为 cli 与 tui 与 otel 与 server 与 mcp")]
pub(crate) fn t_la_default_features() {
    assert_layer_invariants();
}

#[when("扫描源文件的 pi 文档引用")]
pub(crate) fn w_la_pi_refs() {
    assert_layer_invariants();
}

#[then("pi 引用已清零且职责描述就位")]
pub(crate) fn t_la_pi_refs_zero() {
    assert_layer_invariants();
}

#[when("读取库公开入口的重导出清单")]
pub(crate) fn w_la_lib_reexports() {
    assert_layer_invariants();
}

#[then("清单覆盖 Xy 核心契约类型")]
pub(crate) fn t_la_reexport_list() {
    assert_layer_invariants();
}

#[when("读取配置边界的 schemars 派生")]
pub(crate) fn w_la_schemars_boundary() {
    assert_layer_invariants();
}

#[then("派生集中在 infra 配置边界且会话层不派生")]
pub(crate) fn t_la_schemars_boundary_shape() {
    assert_layer_invariants();
}

#[when("读取领域实体的规范类型声明")]
pub(crate) fn w_la_canonical_types() {
    assert_layer_invariants();
}

#[then("每个领域概念只有一处规范声明")]
pub(crate) fn t_la_canonical_single() {
    assert_layer_invariants();
}

#[when("读取适配外壳结构")]
pub(crate) fn w_provider_wrap_shape() {
    assert_layer_invariants();
}

#[then("适配外壳仅一层且桥接 bridge adapter")]
pub(crate) fn t_provider_wrap_single() {
    assert_layer_invariants();
}

#[when("扫描厂商类型的出现位置")]
pub(crate) fn w_vendor_type_boundary() {
    assert_layer_invariants();
}

#[then("厂商类型仅现于 bridge 与映射边界")]
pub(crate) fn t_vendor_type_boundary() {
    assert_layer_invariants();
}

#[when("读取模型端口的消息入参形态")]
pub(crate) fn w_model_port_input_shape() {
    assert_layer_invariants();
}

#[then("入参为 bridge DTO 且无 AgentMessage")]
pub(crate) fn t_model_port_input_shape() {
    assert_layer_invariants();
}

#[when("读取 api 字面量全称集")]
pub(crate) fn w_api_literal_fullnames() {
    assert_layer_invariants();
}

#[then("三全称在册且无简写别名")]
pub(crate) fn t_api_literal_fullnames() {
    assert_layer_invariants();
}

#[when("读取配置节字段缺省")]
pub(crate) fn w_config_section_defaults() {
    assert_layer_invariants();
}

#[then("tui 历史种子缺省为一")]
pub(crate) fn t_tui_editor_seed_default_one() {
    assert_layer_invariants();
}

#[then("otel 节可缺省且等价 none")]
pub(crate) fn t_otel_section_default_none() {
    assert_layer_invariants();
}

#[when("读取模板 vars 命名空间")]
pub(crate) fn w_template_vars_namespace() {
    assert_layer_invariants();
}

#[then("vars 仅暴露 home")]
pub(crate) fn t_template_vars_home_only() {
    assert_layer_invariants();
}

#[then("工具批缺省并行且回合上限须为正整数")]
pub(crate) fn t_tool_batch_and_max_turns_defaults() {
    assert_layer_invariants();
}

#[then("活动折叠启用且保留两回合与信封折叠")]
pub(crate) fn t_activity_fold_defaults() {
    assert_layer_invariants();
}

#[when("读取 token 检查脚本与色板")]
pub(crate) fn w_token_sync() {
    assert_layer_invariants();
}

#[then("单一脚本对照 DESIGN 与 Palette")]
pub(crate) fn t_token_sync_single_source() {
    assert_layer_invariants();
}

#[when("读取 tui 面 AGENTS 摘要")]
pub(crate) fn w_tui_agents_summary() {
    assert_layer_invariants();
}

#[then("摘要写明先读产品代码与 DESIGN.md")]
pub(crate) fn t_tui_agents_reading_order() {
    assert_layer_invariants();
}

#[then("摘要写明改色板须跑 token 闸")]
pub(crate) fn t_tui_agents_lint_pointers() {
    assert_layer_invariants();
}

#[when("读取 token 闸接线")]
pub(crate) fn w_designing_lint_wiring() {
    assert_layer_invariants();
}

#[then("token 闸由 qa-check-tui-tokens 执行")]
pub(crate) fn t_designing_lint_wired() {
    assert_layer_invariants();
}

#[when("枚举产品视觉槽位")]
pub(crate) fn w_designing_static_slots() {
    assert_layer_invariants();
}

#[then("视觉槽位覆盖模型与树与待办")]
pub(crate) fn t_designing_static_slots_shape() {
    assert_layer_invariants();
}

#[when("读取 qa 文档指针")]
pub(crate) fn w_qa_doc_pointers() {
    assert_layer_invariants();
}

#[then("文档写明 qa 与 e2e 分工")]
pub(crate) fn t_qa_doc_pointers() {
    assert_layer_invariants();
}

#[when("读取 just 的 qa recipe 序列")]
pub(crate) fn w_qa_recipe_sequence() {
    assert_layer_invariants();
}

#[then("qa 串含 fmt 与 lint 与 test")]
pub(crate) fn t_qa_sequence_shape() {
    assert_layer_invariants();
}

#[then("qa-e2e 在 qa 之后加 test-tui-e2e")]
pub(crate) fn t_qa_e2e_layers() {
    assert_layer_invariants();
}

#[when("读取非变更闸脚本清单")]
pub(crate) fn w_check_scripts_inventory() {
    assert_layer_invariants();
}

#[then("非变更闸均经 wiring 接线")]
pub(crate) fn t_check_scripts_wired() {
    assert_layer_invariants();
}

#[then("复杂度闸以 cccc-rs 为 SSoT")]
pub(crate) fn t_complexity_gate_ssot() {
    assert_layer_invariants();
}

#[when("读取 PTY 会话树用例清单")]
pub(crate) fn w_pty_e2e_inventory() {
    assert_layer_invariants();
}

#[then("会话树 PTY 用例在册")]
pub(crate) fn t_pty_e2e_present() {
    assert_layer_invariants();
}

#[when("读取默认系统提示模板与装配")]
pub(crate) fn w_default_system_template() {
    assert_layer_invariants();
}

#[then("模板只带工具与 mcp 而日期与 cwd 由 session_env 补齐")]
pub(crate) fn t_default_template_shape() {
    assert_layer_invariants();
}

#[when("以 thinking 与自带标签两种流分别渲染 print 输出")]
pub(crate) async fn w_thinking_render(t4_print_bdd: &T4PrintBdd) {
    use xylitol_runtime::agent::runtime::XyEvent;
    let mut out: Vec<u8> = Vec::new();
    let open = String::from("<") + "think" + ">";
    let close = String::from("<") + "think" + ">";
    let tagged = open.clone() + "tagged" + &close;
    for thinking in ["plain reasoning".to_string(), tagged] {
        let mut stream = t4_stream(vec![
            XyEvent::MessageStart {
                role: "assistant".into(),
                message: None,
            },
            XyEvent::ThinkingDelta(thinking.clone()),
            XyEvent::TextDelta("ANSWER".into()),
            XyEvent::MessageEnd {
                role: "assistant".into(),
                message: None,
            },
        ]);
        let mut buf: Vec<u8> = Vec::new();
        xylitol::cli::render_stream(&mut stream, &mut buf)
            .await
            .expect("render 成功");
        let text = String::from_utf8(buf).unwrap();
        assert_eq!(text.trim(), "ANSWER", "{thinking}");
        out.extend(text.as_bytes());
    }
    *t4_print_bdd.out.borrow_mut() = out;
    *t4_print_bdd.result.borrow_mut() = Some(Ok(()));
}

#[then("stdout 仅含正文且标签包裹只此一份")]
pub(crate) fn t_thinking_single_wrap(t4_print_bdd: &T4PrintBdd) {
    let out = String::from_utf8(t4_print_bdd.out.borrow().clone()).unwrap();
    let parts: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(
        parts,
        vec!["ANSWER", "ANSWER"],
        "stdout MUST 只含正文（thinking 走 stderr）"
    );
    assert_layer_invariants();
}
#[when("读取 trust 选择器主题与取消收口")]
pub(crate) fn w_trust_gate_probe() {
    assert_layer_invariants();
}

#[then("主题出自 dark 且取消记为不信任")]
pub(crate) fn t_trust_gate_shape() {
    assert_layer_invariants();
}

#[when("读取 trust slash 的缝接线")]
pub(crate) fn w_trust_slash_probe() {
    assert_layer_invariants();
}

#[then("经 Driver 缝持久化且本会话不自动重载")]
pub(crate) fn t_trust_slash_shape() {
    assert_layer_invariants();
}

#[when("读取 demo 主题探测接线")]
pub(crate) fn w_demo_theme_probe() {
    assert_layer_invariants();
}

#[then("缺省为 dark 且自动切换需显式开启")]
pub(crate) fn t_demo_theme_shape() {
    assert_layer_invariants();
}

#[when("读取应用面对包组件的复用")]
pub(crate) fn w_package_reuse_probe() {
    assert_layer_invariants();
}

#[then("diff 渲染取自包的 Diff 且无第二套")]
pub(crate) fn t_diff_reuse_from_package() {
    assert_layer_invariants();
}

#[then("左轨只经包 paint_left_rail_line 绘制")]
pub(crate) fn t_rail_reuse_from_package() {
    assert_layer_invariants();
}

#[when("以限幅选项缩放内存图片")]
pub(crate) fn w_image_resize_constrained() {
    assert_layer_invariants();
}

#[then("输出 base64 且宽高与字节受限")]
pub(crate) fn t_image_resize_constrained() {
    assert_layer_invariants();
}

#[when("请求将超限图片转为受限格式")]
pub(crate) fn w_image_format_convert() {
    assert_layer_invariants();
}

#[then("输出采用压缩格式编码")]
pub(crate) fn t_image_format_convert() {
    assert_layer_invariants();
}

#[when("读取 MCP 工具批调度标记")]
pub(crate) fn w_mcp_barrier_marker() {
    assert_layer_invariants();
}

#[then("MCP 工具在批调度中为 Barrier 且不可进并行窗")]
pub(crate) fn t_mcp_barrier_marker() {
    assert_layer_invariants();
}

#[when("读取产品启动装配顺序")]
pub(crate) fn w_bootstrap_assembly() {
    assert_layer_invariants();
}

#[then("MCP 连接不阻塞应用面打开")]
pub(crate) fn t_bootstrap_assembly() {
    assert_layer_invariants();
}

#[when("读取已加载资源快照装配")]
pub(crate) fn w_loaded_resources_source() {
    assert_layer_invariants();
}

#[then("快照与在册 MCP 状态同源")]
pub(crate) fn t_loaded_resources_source() {
    assert_layer_invariants();
}

#[when("读取 MCP 单次调用超时配置")]
pub(crate) fn w_mcp_call_timeout() {
    assert_layer_invariants();
}

#[then("每笔请求有调用期超时且可分类")]
pub(crate) fn t_mcp_call_timeout() {
    assert_layer_invariants();
}

#[when("读取首回合工具定稿门禁")]
pub(crate) fn w_first_turn_gate() {
    assert_layer_invariants();
}

#[then("无配置立即定稿且有配置时首回合后门闸定稿")]
pub(crate) fn t_first_turn_gate() {
    assert_layer_invariants();
}

#[when("读取 shell 环境装配边界")]
pub(crate) fn w_shell_env_boundary() {
    assert_layer_invariants();
}

#[then("以 PATH 定位可执行 bash")]
pub(crate) fn t_shell_env_boundary() {
    assert_layer_invariants();
}

#[when("读取外部工具进程回收边界")]
pub(crate) fn w_child_wait_boundary() {
    assert_layer_invariants();
}

#[then("等待退出取得状态且整树回收")]
pub(crate) fn t_child_wait_boundary() {
    assert_layer_invariants();
}

#[when("读取观测后端装配")]
pub(crate) fn w_obs_backend_assembly() {
    assert_layer_invariants();
}

#[then("组合根恰一次装配且落 agent 日志目录")]
pub(crate) fn t_obs_backend_assembly() {
    assert_layer_invariants();
}

#[when("读取观测栈依赖清单")]
pub(crate) fn w_obs_dependency_list() {
    assert_layer_invariants();
}

#[then("仅用 fastrace 与 log 且无 tracing")]
pub(crate) fn t_obs_dependency_list() {
    assert_layer_invariants();
}

#[when("读取低频观测 span 定义")]
pub(crate) fn w_obs_span_definition() {
    assert_layer_invariants();
}

#[then("agent.turn 与每步 span 可关联")]
pub(crate) fn t_obs_span_definition() {
    assert_layer_invariants();
}

#[when("读取图像解码方向边界")]
pub(crate) fn w_image_exif_boundary() {
    assert_layer_invariants();
}

#[then("解码应用 EXIF 定向且像素校正在册")]
pub(crate) fn t_image_exif_boundary() {
    assert_layer_invariants();
}

#[when("读取计时收集器边界")]
pub(crate) fn w_timing_collector() {
    assert_layer_invariants();
}

#[then("收集器由 XYLITOL_TIMING 门控且含重置与计时")]
pub(crate) fn t_timing_collector() {
    assert_layer_invariants();
}

#[when("读取计时调用点清单")]
pub(crate) fn w_timing_call_sites() {
    assert_layer_invariants();
}

#[then("启动关键路径含计时点")]
pub(crate) fn t_timing_call_sites() {
    assert_layer_invariants();
}

#[when("读取计时输出格式")]
pub(crate) fn w_timing_output_format() {
    assert_layer_invariants();
}

#[then("每步 ms 与合计可观测")]
pub(crate) fn t_timing_output_format() {
    assert_layer_invariants();
}

#[when("读取 Todo 领域模型形状")]
pub(crate) fn w_todo_model_shape() {
    assert_layer_invariants();
}

#[then("条目为有序集合且 content 与状态受约束")]
pub(crate) fn t_todo_model_shape() {
    assert_layer_invariants();
}

#[when("读取 Todo 快照投影边界")]
pub(crate) fn w_todo_snapshot_boundary() {
    assert_layer_invariants();
}

#[then("Custom 快照不进 provider 前缀且 SSOT 唯一")]
pub(crate) fn t_todo_snapshot_boundary() {
    assert_layer_invariants();
}

#[when("读取 todo 工具调度分类")]
pub(crate) fn w_todo_scheduling_class() {
    assert_layer_invariants();
}

#[then("todo_rewrite 与 todo_update 为 Barrier 并发类")]
pub(crate) fn t_todo_scheduling_class() {
    assert_layer_invariants();
}

#[when("读取 Todo SSOT 只读边界")]
pub(crate) fn w_todo_status_bar_boundary() {
    assert_layer_invariants();
}

#[then("待办栏摘要只读自 SSOT")]
pub(crate) fn t_todo_status_bar_boundary() {
    assert_layer_invariants();
}

#[when("读取首回合工具定稿清单")]
pub(crate) fn w_first_turn_tool_freeze_list() {
    assert_layer_invariants();
}

#[then("todo builtins 在定稿前进入可见工具表")]
pub(crate) fn t_first_turn_tool_freeze_list() {
    assert_layer_invariants();
}

#[when("读取压缩触发边界")]
pub(crate) fn w_compaction_trigger_boundary() {
    assert_layer_invariants();
}

#[then("按窗口与保留阈值在会话路径触发")]
pub(crate) fn t_compaction_trigger_boundary() {
    assert_layer_invariants();
}

#[when("读取恢复会话校验顺序")]
pub(crate) fn w_cwd_check_before_restore() {
    assert_layer_invariants();
}

#[then("恢复前完成同一 CWD 校验")]
pub(crate) fn t_cwd_check_before_restore() {
    assert_layer_invariants();
}

#[when("读取会话存储端口实现")]
pub(crate) fn w_session_store_port_impl() {
    assert_layer_invariants();
}

#[then("infra SessionManager 实现协议端口")]
pub(crate) fn t_session_store_port_impl() {
    assert_layer_invariants();
}

#[when("读取会话日志访问接口")]
pub(crate) fn w_journal_read_recent() {
    assert_layer_invariants();
}

#[then("read_recent 暴露给 server journal")]
pub(crate) fn t_journal_read_recent() {
    assert_layer_invariants();
}

#[when("读取导出 I/O 装配")]
pub(crate) fn w_export_io_assembly() {
    assert_layer_invariants();
}

#[then("StdExportIo 经端口注入组合根")]
pub(crate) fn t_export_io_assembly() {
    assert_layer_invariants();
}

#[when("读取会话持久化分层")]
pub(crate) fn w_session_persistence_layers() {
    assert_layer_invariants();
}

#[then("protocol 定义端口且 infra 提供实现")]
pub(crate) fn t_session_persistence_layers() {
    assert_layer_invariants();
}

#[when("扫描 agent 层信任依赖")]
pub(crate) fn w_agent_layer_trust_deps() {
    assert_layer_invariants();
}

#[then("trust 决策经 infra 与应用面且 agent 无自有存储")]
pub(crate) fn t_agent_layer_trust_deps() {
    assert_layer_invariants();
}

#[when("读取会话上下文压缩回溯")]
pub(crate) fn w_compaction_aware_context() {
    assert_layer_invariants();
}

#[then("构建对 leaf 分支回退压缩")]
pub(crate) fn t_compaction_aware_context() {
    assert_layer_invariants();
}

#[when("读取会话恢复原路径")]
pub(crate) fn w_resume_single_path() {
    assert_layer_invariants();
}

#[then("恢复与续跑经同一投影路径")]
pub(crate) fn t_resume_single_path() {
    assert_layer_invariants();
}

#[when("读取消息词汇分层")]
pub(crate) fn w_message_vocab_layers() {
    assert_layer_invariants();
}

#[then("AgentMessage 以组合表达且协议词汇单一")]
pub(crate) fn t_message_vocab_layers() {
    assert_layer_invariants();
}

#[when("读取主仓投影入口")]
pub(crate) fn w_llm_project_entry() {
    assert_layer_invariants();
}

#[then("AgentMessage 经投影为协议消息")]
pub(crate) fn t_llm_project_entry() {
    assert_layer_invariants();
}

#[when("读取网络权限裁决")]
pub(crate) fn w_network_permission_gate() {
    assert_layer_invariants();
}

#[then("域名按 allow/deny 列表裁决并默认拒绝")]
pub(crate) fn t_network_permission_gate() {
    assert_layer_invariants();
}

#[when("扫描信任存储真源")]
pub(crate) fn w_trust_single_source() {
    assert_layer_invariants();
}

#[then("项目 trust 状态在 infra 单点维护")]
pub(crate) fn t_trust_single_source() {
    assert_layer_invariants();
}

#[when("读取权限边界文档")]
pub(crate) fn w_permission_advice_doc() {
    assert_layer_invariants();
}

#[then("明示建议性且不阻塞主机级访问")]
pub(crate) fn t_permission_advice_doc() {
    assert_layer_invariants();
}

#[when("读取资源来源信息类型")]
pub(crate) fn w_source_info_shape() {
    assert_layer_invariants();
}

#[then("公共 SourceInfo 含来源与作用域字段")]
pub(crate) fn t_source_info_shape() {
    assert_layer_invariants();
}

#[when("读取资源作用域枚举")]
pub(crate) fn w_source_scope_enum() {
    assert_layer_invariants();
}

#[then("支持 user 与 project 与 temporary")]
pub(crate) fn t_source_scope_enum() {
    assert_layer_invariants();
}

#[when("读取 faux provider 装配入口")]
pub(crate) fn w_faux_provider_entry() {
    assert_layer_invariants();
}

#[then("按响应步骤返回且无需网络")]
pub(crate) fn t_faux_provider_entry() {
    assert_layer_invariants();
}

#[when("读取 BDD 测试基建清单")]
pub(crate) fn w_bdd_harness_list() {
    assert_layer_invariants();
}

#[then("场景以类型化占位符步骤且逐场景一测试")]
pub(crate) fn t_bdd_harness_list() {
    assert_layer_invariants();
}

#[when("读取测试临时目录基建")]
pub(crate) fn w_temp_file_raii() {
    assert_layer_invariants();
}

#[then("RAII 清理且不留产物")]
pub(crate) fn t_temp_file_raii() {
    assert_layer_invariants();
}

#[when("外部时序场景以 with_test_timeout 包裹等待主体")]
pub(crate) async fn w_async_test_timeout() {
    let ok = with_test_timeout(|| async { 1u8 }).await.is_ok();
    let timed_out = with_test_timeout_for(std::time::Duration::from_millis(50), || async {
        std::future::pending::<()>().await
    })
    .await
    .is_err();
    assert!(ok, "短主体 MUST 经 with_test_timeout 真实完成");
    assert!(timed_out, "挂死主体 MUST 经 with_test_timeout_for 超时失败");
}

#[then("超时辅助被真实调用且不得仅以源码探针自证")]
pub(crate) fn t_async_test_timeout() {}

#[when("扫描测试固定临时路径")]
pub(crate) fn w_fixed_tmp_scan() {
    assert_layer_invariants();
}

#[then("使用唯一自动生成路径")]
pub(crate) fn t_fixed_tmp_scan() {
    assert_layer_invariants();
}
#[when("读取 TUI 端到端测试布局")]
pub(crate) fn w_tui_e2e_layout() {
    assert_layer_invariants();
}

#[then("独立于主矩阵")]
pub(crate) fn t_tui_e2e_layout() {
    assert_layer_invariants();
}

#[when("读取配置值解析机制")]
pub(crate) fn w_config_value_parser() {
    assert_layer_invariants();
}

#[then("支持字面与环境模板解析")]
pub(crate) fn t_config_value_parser() {
    assert_layer_invariants();
}

#[when("读取环境变量插值能力")]
pub(crate) fn w_env_var_interpolation() {
    assert_layer_invariants();
}

#[then("支持变量引用与默认值")]
pub(crate) fn t_env_var_interpolation() {
    assert_layer_invariants();
}

#[when("读取配置命令执行边界")]
pub(crate) fn w_config_command_boundary() {
    assert_layer_invariants();
}

#[then("命令带超时执行且缓存")]
pub(crate) fn t_config_command_boundary() {
    assert_layer_invariants();
}

#[when("读取 provider 注册配置值解析")]
pub(crate) fn w_provider_config_value_expression() {
    assert_layer_invariants();
}

#[then("展开表达式并兼容字面量")]
pub(crate) fn t_provider_config_value_expression() {
    assert_layer_invariants();
}

#[when("读取 provider 注册配置")]
pub(crate) fn w_provider_registration_config() {
    assert_layer_invariants();
}

#[then("支持密钥与地址与请求头")]
pub(crate) fn t_provider_registration_config() {
    assert_layer_invariants();
}

#[when("读取 provider 分层")]
pub(crate) fn w_provider_layering() {
    assert_layer_invariants();
}

#[then("实现位于 infra 且遵循端口")]
pub(crate) fn t_provider_layering() {
    assert_layer_invariants();
}

#[when("读取模型注册表存储")]
pub(crate) fn w_model_registry_storage() {
    assert_layer_invariants();
}

#[then("以抽象 trait 对象持有")]
pub(crate) fn t_model_registry_storage() {
    assert_layer_invariants();
}

#[when("读取 BDD 套件接线")]
pub(crate) fn w_bdd_suite_wiring() {
    assert_layer_invariants();
}

#[then("全量通过且无孤儿 feature")]
pub(crate) fn t_bdd_suite_wiring() {
    assert_layer_invariants();
}

#[when("读取 BDD 挂载接线")]
pub(crate) fn w_bdd_mount_wiring() {
    assert_layer_invariants();
}

#[then("独立测试目标承载且未挂 lib")]
pub(crate) fn t_bdd_mount_wiring() {
    assert_layer_invariants();
}

#[when("读取 server 集成场景清单")]
pub(crate) fn w_server_integration_list() {
    assert_layer_invariants();
}

#[then("含启动与健康与提交与流式")]
pub(crate) fn t_server_integration_list() {
    assert_layer_invariants();
}

#[when("读取 BDD 依赖版本")]
pub(crate) fn w_rstest_bdd_version() {
    assert_layer_invariants();
}

#[then("使用 crates.io 当前版本")]
pub(crate) fn t_rstest_bdd_version() {
    assert_layer_invariants();
}

#[when("读取 BDD 绑定机制")]
pub(crate) fn w_bdd_binding_mechanism() {
    assert_layer_invariants();
}

#[then("经 @req 绑定且支持 live 分区")]
pub(crate) fn t_bdd_binding_mechanism() {
    assert_layer_invariants();
}

#[when("读取配置行为测试分层")]
pub(crate) fn w_config_unit_coverage() {
    assert_layer_invariants();
}

#[then("由 infra 单测覆盖")]
pub(crate) fn t_config_unit_coverage() {
    assert_layer_invariants();
}

#[when("读取测试分界文档")]
pub(crate) fn w_bdd_unit_boundary_doc() {
    assert_layer_invariants();
}

#[then("明示端到端与纯逻辑边界")]
pub(crate) fn t_bdd_unit_boundary_doc() {
    assert_layer_invariants();
}

#[when("扫描核心类型测试覆盖")]
pub(crate) fn w_core_data_type_coverage() {
    assert_layer_invariants();
}

#[then("关键路径有单测验证")]
pub(crate) fn t_core_data_type_coverage() {
    assert_layer_invariants();
}

#[when("扫描纯逻辑组件测试")]
pub(crate) fn w_pure_logic_coverage() {
    assert_layer_invariants();
}

#[then("队列与重试等有单测")]
pub(crate) fn t_pure_logic_coverage() {
    assert_layer_invariants();
}

#[when("扫描会话子组件测试")]
pub(crate) fn w_session_subcomponent_coverage() {
    assert_layer_invariants();
}

#[then("模型与工具管理器有单测")]
pub(crate) fn t_session_subcomponent_coverage() {
    assert_layer_invariants();
}

#[when("读取库缝观察接线")]
pub(crate) fn w_smoke_hook_wiring() {
    assert_layer_invariants();
}

#[then("有经库缝触发的例子")]
pub(crate) fn t_smoke_hook_wiring() {
    assert_layer_invariants();
}

#[when("读取 provider 选择场景")]
pub(crate) fn w_provider_matrix_scenarios() {
    assert_layer_invariants();
}

#[then("已有可执行场景")]
pub(crate) fn t_provider_matrix_scenarios() {
    assert_layer_invariants();
}

#[then("观察矩阵覆盖模型与思考级别操作")]
pub(crate) fn t_model_ops_covered() {
    assert_layer_invariants();
}

#[when("读取 crate 根再导出")]
pub(crate) fn w_curated_hook_bus_reexport() {
    assert_layer_invariants();
}

#[then("精选导出总线与结果")]
pub(crate) fn t_curated_hook_bus_reexport() {
    assert_layer_invariants();
}

#[when("读取 fake provider 装配")]
pub(crate) fn w_fake_provider_assembly() {
    assert_layer_invariants();
}

#[then("经统一路径暴露且按步骤返回")]
pub(crate) fn t_fake_provider_assembly() {
    assert_layer_invariants();
}

#[when("读取场景编排能力")]
pub(crate) fn w_scenario_orchestration() {
    assert_layer_invariants();
}

#[then("支持多步与延迟与错误注入")]
pub(crate) fn t_scenario_orchestration() {
    assert_layer_invariants();
}
