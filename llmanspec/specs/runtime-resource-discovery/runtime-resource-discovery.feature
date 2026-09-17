# language: zh-CN
# capability: runtime-resource-discovery
# purpose: 统一资源发现：skills、themes、AGENTS.md / SYSTEM 与诊断（list/info/doctor）；不含 slash prompt 模板。
# scope: src/infra/resource/, src/protocol/

功能: runtime-resource-discovery

  @req:r1759 @human
  场景: 资源列表
    - System MUST 提供只读命令，列出全部已发现 skills、themes 及其 source scope 与 path；MUST NOT 将 prompts/*.md 列为可展开 prompt 模板资源。

  @req:r1763 @human
  场景: 资源详情
    - System MUST 提供命令展示单个命名资源详情，未找到时以非零退出。

  @req:r1764 @human
  场景: 资源 doctor
    - System MUST 提供命令展示 ResourceLoader 诊断，存在问题时以非零退出。

  @req:r1765 @human
  场景: 只读
    - 资源命令 MUST NOT 创建、修改或删除任何文件或已安装包状态。

  @req:r1766 @human
  场景: 复用 loader
    - 资源命令 MUST 复用 DefaultResourceLoader 发现，而非独立重扫。

  @req:r1767 @human
  场景: 统一 SourceInfo 类型
    - System MUST 提供公共 SourceInfo 类型，含 path、source、scope、origin、base_dir 字段。

  @req:r1768 @human
  场景: Source 工厂
    - System MUST 提供 create_source_info 与 create_synthetic_source_info 工厂函数。

  @req:r1769 @human
  场景: Source 迁移
    - Skills、SlashCommandInfo MUST 使用统一 SourceInfo 类型；MUST NOT 再要求 PromptTemplate 类型。

  @req:r1770 @human
  场景: Scope 枚举
    - SourceScope MUST 支持 user、project、temporary 变体，对齐 pi。

  @req:r1760 @human
  场景: 资源热重载
    - DefaultResourceLoader MUST 提供 reload（或实现 XyReloadable）：清空缓存后重新发现 context/skills/themes/SYSTEM/APPEND；失败诊断 MUST 可观察；reload MUST NOT 修改会话历史文件；MUST NOT 发现 prompts 为 slash 模板。

  @req:r1761 @human
  场景: 主题名发现
    - System MUST 能在 Trust 语义下经 resource loader（get_themes 或等价）列出已发现 theme 文件名（stem）；未信任 MUST NOT 列出项目 themes 目录条目。应用面 MAY 仅暴露内建主题切换；发现能力 MUST 仍可由 loader 路径验证。

  @req:r1762 @human
  场景: Skills 发现与重载报告
    - System MUST 能在 Trust 语义下发现并列出已加载 skills（名与 scope）；提供 reload_skills（或等价）清空后重扫并返回 report（names/count；诊断可观察）；未信任 MUST NOT 列入项目 .xylitol/skills；reload MUST NOT 修改会话历史文件。
