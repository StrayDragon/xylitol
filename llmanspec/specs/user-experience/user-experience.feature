# language: zh-CN
# purpose: 用户体验 — 鉴权引导消息、错误消息、启动诊断与产品展示名。

功能: user-experience

  @req:r1838
  规则: login-help
    System 须提供登录引导消息（或等价入口），返回引用 /login 与文档路径的引导。

    场景: 登录引导引用文档
      那么 登录引导含 /login 与文档路径
  @req:r1839
  规则: no-models
    无可用模型时，System 须提供无模型提示消息（或等价入口）。

    场景: 无模型引导
      那么 无可用模型提示衔接登录引导
  @req:r1840
  规则: no-model-selected
    未选择模型时，System 须提供未选模型提示消息（或等价入口）。

    场景: 未设模型显示
      那么 未选模型展示占位而非厂商默认名
  @req:r1841
  规则: no-api-key
    System 须提供无 API key 提示消息（或等价入口），含 provider 名称。
    # （c2827 合并：runtime-model-registry r1756 鉴权指引并入本条。）

    场景: 无API键名称厂商
      那么 无 api key 提示含 provider 名

  @req:r3065
  规则: ensure-host-diagnostics
    确保 Host 失败时必须给出可操作诊断，区分拉起失败、就绪超时、端口被非 xylitol 占用与版本不符；不得只提示先手 xylitol serve。

    场景: 确保失败诊断可操作
      当 读取拉起失败与就绪超时与非 xylitol 占用的诊断
      那么 各诊断可操作且不只提示先手 serve
  @req:r3066
  规则: product-display-identity
    产品给人看的名字必须是 XYlitol（前两字母大写、其余小写）；命令名、crate、配置目录、数据目录与环境变量必须保持 xylitol 或 XYLITOL_*。面向人的文案（TUI 头卡、README 标题、CLI about）必须用 XYlitol，不得把 xylitol 当作产品展示名，不得展示木糖醇中文标签。
    场景: CLI帮助展示产品名
      当 xylitol --help
      那么 帮助文案含 XYlitol 且命令名为 xylitol

  @req:r3075
  规则: config-unknown-key-errors
    配置未知顶层键或旧键导致硬失败时，错误 须 指出键名与所属配置节，并给出迁移方向；`xylitol.unknown_keys: warn` 时 须 以警告列出同一信息。不得 仅输出泛化解析错误。

    场景: 未知键错误可操作
      当 加载含未知顶层键的配置
      那么 错误含键名与迁移提示
