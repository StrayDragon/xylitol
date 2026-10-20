# language: zh-CN
# purpose: 统一资源发现：skills、themes、AGENTS.md / SYSTEM；不含 slash prompt 模板；不含 CLI resources 诊断命令。

功能: runtime-resource-discovery

  @req:r1759
  规则: 资源列表
    System 须经资源加载器列出已发现 skills、themes 及其 source scope 与 path；不得将 prompts/*.md 列为可展开 prompt 模板资源。

    场景: 资源列表技能主题无提示词
      假如 项目或全局 prompts 目录存在 greet.md
      当 装配资源加载器并发现
      那么 列表含 skills 与 themes 且不含 prompts
  @req:r1767
  规则: 统一 SourceInfo 类型
    System 须提供公共 SourceInfo 类型，含 path、source、scope、origin、base_dir 字段。
    # （c2827 合并：r1768 Source 工厂与 r1769 Source 迁移均属统一 SourceInfo 类型的子条款，并入本条承载。）

    场景: 来源信息公开形状
      当 读取资源来源信息类型
      那么 公共 SourceInfo 含来源与作用域字段
  @req:r1768
  规则: Scope 枚举
    SourceScope 须支持 user、project、temporary 变体，对齐 pi。

    场景: 作用域枚举变体
      当 读取资源作用域枚举
      那么 支持 user 与 project 与 temporary
  @req:r1760
  规则: 资源热重载
    DefaultResourceLoader 须提供 reload（或实现 XyReloadable）：清空缓存后重新发现 context/skills/themes/SYSTEM/APPEND；失败诊断 须可观察；reload 不得修改会话历史文件；不得发现 prompts 为 slash 模板。

    场景: 上下文热重载信任门控
      当 在项目目录写 AGENTS.md 并分别以信任与未信任重载 context
      那么 信任时项目 context 注入而未信任时被跳过且会话条目不变
  @req:r1761
  规则: 主题名发现
    System 须能在 Trust 语义下经 resource loader（get_themes 或等价）列出已发现 theme 文件名（stem）；未信任 不得列出项目 themes 目录条目。应用面 MAY 仅暴露内建主题切换；发现能力 须仍可由 loader 路径验证。

    场景: 主题列出按词干
      假如 资源目录含 demo 技能与 dark 主题
      当 经 loader 列出已发现资源
      那么 列出技能名与主题 stem
  @req:r1762
  规则: Skills 发现与重载报告
    System 须能在 Trust 语义下发现并列出已加载 skills（名与 scope）；提供 reload_skills（或等价）清空后重扫并返回 report（names/count；诊断可观察）；未信任 不得列入项目 .xylitol/skills；reload 不得修改会话历史文件。

    场景: 技能热重载信任门控
      当 在项目目录写 skill 并分别以信任与未信任重载 skills
      那么 信任时项目 skill 注入且可查询已加载名而未信任时被跳过
