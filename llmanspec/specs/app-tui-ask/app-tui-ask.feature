# language: zh-CN
# capability: app-tui-ask
# purpose: 产品 TUI 内置工具 ask：澄清/分叉问卷、仅 TUI 装配、Choice 槽与 scrollback 人话摘要。
# scope: src/app/tui/

功能: app-tui-ask

  @req:r1167
  规则: tui-only-registration
    产品 TUI 装配的 ToolSet MUST 含内置工具 ask；Print 与非 TUI composition MUST NOT 注册 ask。

    场景: tui-registers-ask
      假如 产品以 TUI 面启动
      当 查询装配后的 ToolSet
      那么 含名为 ask 的内置工具

    场景: print-omits-ask
      假如 产品以 Print 面启动
      当 查询装配后的 ToolSet
      那么 不含名为 ask 的工具
  @req:r1168
  规则: choice-slot-mount
    当 ask 工具执行等待用户时，产品 TUI MUST 在 editor 槽挂载 ChoicePrompt（解冻 EditorSlot::Choice）；MUST NOT 使用 stderr 数字菜单。

    场景: mounts-choice-slot
      假如 ask 工具已开始等待用户
      当 产品 TUI 渲染 editor 槽
      那么 槽为 ChoicePrompt 且标题含 Ask
  @req:r1169
  规则: skip-success
    用户 Skip 或 Esc MUST 使 ask 以成功结构化结果返回（status=skipped）；MUST NOT 将 skip 表示为 tool error；Abort/取消整轮 MUST NOT 伪装为 skip。

    场景: esc-skips-success
      假如 ChoicePrompt 因 ask 打开
      当 按 Esc
      那么 ask 工具结果为 status skipped 成功 JSON
  @req:r1170
  规则: answered-payload
    用户提交 MUST 返回 status=answered 与 answers（含 question id 与所选 values/labels）；LLM 消费 JSON；人看 scrollback 人话。

    场景: submit-answered
      假如 ChoicePrompt 因 ask 打开且用户已选选项
      当 提交
      那么 ask 工具结果为 status answered 且含 answers
  @req:r1171
  规则: scrollback-ask-rail
    ask 相关 scrollback MUST 使用固定左边轨与人话摘要（waiting/answered/skipped 语义色）；MUST NOT 默认以 tool-*-bg 洗底或 raw JSON 作为人对人展示。

    场景: scrollback-human-rail
      假如 ask 已结束（answered 或 skipped）
      当 渲染 scrollback
      那么 出现 Ask 人话摘要与左边轨且无 raw tool JSON 洗底
  @req:r1172
  规则: not-trust
    ask MUST NOT 替代 Trust 闸；Trust 仍走 app-tui-trust / bootstrap ChoicePrompt。

    场景: trust-untouched
      假如 项目尚未信任
      当 启动需 Trust 闸的路径
      那么 仍走 Trust ChoicePrompt 而非 ask 工具
# re-review(c2826): 复审结论——本 capability 管辖行为不变；分支内改动仅测试基建与可见性再导出（2026-09-28）

# re-review(c2827): 复审结论——本 capability 管辖行为不变；分支内改动为 BDD 场景落地、BDD 测试基建（steps/bindings/驱动旋钮与探针）与可见性再导出（2026-09-28）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）

# re-review(c2838): c2838 intra-doc 链接治理触及本 scope 内源码 doc 注释（纯文档、无行为变化）。场景映射不变量保持；已复核。（2026-10-06）
