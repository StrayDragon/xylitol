# language: zh-CN
# capability: runtime-resource-discovery
# purpose: 统一资源发现：skills、themes、AGENTS.md / SYSTEM 与诊断（list/info/doctor）；不含 slash prompt 模板。
# scope: src/infra/resource/, src/protocol/

功能: runtime-resource-discovery

  @req:r1759
  规则: 资源列表
    System MUST 提供只读命令，列出全部已发现 skills、themes 及其 source scope 与 path；MUST NOT 将 prompts/*.md 列为可展开 prompt 模板资源。

    场景: resource-list-skills-themes-no-prompts
      假如 项目或全局 prompts 目录存在 greet.md
      当 装配资源加载器并发现
      那么 列表含 skills 与 themes 且不含 prompts
  @req:r1763
  规则: 资源详情
    System MUST 提供命令展示单个命名资源详情，未找到时以非零退出。

    场景: resource-info-missing-nonzero
      假如 资源目录含 demo 技能与 dark 主题
      当 以资源命令查询不存在的资源
      那么 详情非零退出
  @req:r1764
  规则: 资源 doctor
    System MUST 提供命令展示 ResourceLoader 诊断，存在问题时以非零退出。

    场景: doctor-fails-on-diagnostics
      假如 资源目录含 demo 技能与 dark 主题
      当 以资源命令运行 doctor 且存在不可读技能
      那么 doctor 非零且含诊断
  @req:r1765
  规则: 只读
    资源命令 MUST NOT 创建、修改或删除任何文件或已安装包状态。

    场景: resource-commands-readonly
      假如 资源目录含 demo 技能与 dark 主题
      当 记录资源目录快照
      当 以资源命令列出
      当 以资源命令查询详情 demo-skill
      当 以资源命令运行 doctor
      那么 doctor 零退出
      并且 资源目录快照不变
  @req:r1766
  规则: 复用 loader
    资源命令 MUST 复用 DefaultResourceLoader 发现，而非独立重扫。
    # verified-by: src/app/cli/resources.rs

    场景: resource-commands-reuse-loader
      当 读取资源命令装配
      那么 复用 DefaultResourceLoader 发现
  @req:r1767
  规则: 统一 SourceInfo 类型
    System MUST 提供公共 SourceInfo 类型，含 path、source、scope、origin、base_dir 字段。
    # （c2827 合并：r1768 Source 工厂与 r1769 Source 迁移均属统一 SourceInfo 类型的子条款，并入本条承载。）
    # verified-by: src/protocol/source_info.rs

    场景: source-info-public-shape
      当 读取资源来源信息类型
      那么 公共 SourceInfo 含来源与作用域字段
  @req:r1768
  规则: Scope 枚举
    SourceScope MUST 支持 user、project、temporary 变体，对齐 pi。
    # verified-by: src/protocol/source_info.rs

    场景: scope-enum-variants
      当 读取资源作用域枚举
      那么 支持 user 与 project 与 temporary
  @req:r1760
  规则: 资源热重载
    DefaultResourceLoader MUST 提供 reload（或实现 XyReloadable）：清空缓存后重新发现 context/skills/themes/SYSTEM/APPEND；失败诊断 MUST 可观察；reload MUST NOT 修改会话历史文件；MUST NOT 发现 prompts 为 slash 模板。

    场景: context-hot-reload-trust-gated
      当 在项目目录写 AGENTS.md 并分别以信任与未信任重载 context
      那么 信任时项目 context 注入而未信任时被跳过且会话条目不变
  @req:r1761
  规则: 主题名发现
    System MUST 能在 Trust 语义下经 resource loader（get_themes 或等价）列出已发现 theme 文件名（stem）；未信任 MUST NOT 列出项目 themes 目录条目。应用面 MAY 仅暴露内建主题切换；发现能力 MUST 仍可由 loader 路径验证。

    场景: themes-listed-by-stem
      假如 资源目录含 demo 技能与 dark 主题
      当 以资源命令列出
      那么 退出码为零且输出含技能与主题
  @req:r1762
  规则: Skills 发现与重载报告
    System MUST 能在 Trust 语义下发现并列出已加载 skills（名与 scope）；提供 reload_skills（或等价）清空后重扫并返回 report（names/count；诊断可观察）；未信任 MUST NOT 列入项目 .xylitol/skills；reload MUST NOT 修改会话历史文件。

    场景: skills-hot-reload-trust-gated
      当 在项目目录写 skill 并分别以信任与未信任重载 skills
      那么 信任时项目 skill 注入且可查询已加载名而未信任时被跳过
# re-review(c2835): 复审结论——本 capability 管辖行为不变；仅协议载体常量与死变体清理。（2026-09-29）

# re-review(c2837): c2837 编译隔离变更影响本 scope——agent/infra 公开化与 BDD 测试辅助面收敛（纯可见性扩张与测试基建，无行为变化）。场景映射不变量保持；已复核。（2026-10-06）
# re-review(c2853): 承载分支 sdd/2026-10-review-fixes 触及本 scope（BDD 步骤卫生 / 既有 codec·host 改动）；本 capability 管辖行为不变。场景映射不变量保持。（2026-10-09）
