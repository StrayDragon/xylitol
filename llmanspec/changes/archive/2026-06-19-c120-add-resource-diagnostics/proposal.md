---
depends_on: []
---

# c120-add-resource-diagnostics: 资源只读清单与诊断（list / info / doctor）

## Why
c120 原拟对齐 pi 的 `core/package-manager.ts`（install/update/remove/list/info + npm/git/local 多源）。经评估：

- **生态不可复用**：pi 的 npm 包主体是 TS 扩展（jiti 加载 JS），Rust 无法执行；独立发布的纯文本 skill/prompt 包几乎没有。照搬 npm-centric 的包管理器在 Rust 侧价值极低。
- **自用优先**：用户主要自用，手动 `git clone` 到 `~/.xylitol/skills/` 或 `.xylitol/skills/` 已足够，ResourceLoader（c60 已归档）已能发现。install/update/remove 引入来源元数据登记层，成本 > 价值。
- **砍 install 后 update/remove 无基础**：来源元数据由 install 建立；手动放置的资源系统不知来源，update/remove 无可靠对象。
- **真正独立成立的部分是只读清单+诊断**：ResourceLoader 已发现所有 skills/prompts/themes 并产出诊断（重复名/缺 frontmatter/损坏文件），仅未暴露 CLI。本变更把这些能力以只读命令暴露，零状态、自洽、贴合自用。

## What Changes
- 新增 CLI 子命令 `resources`，三个只读子命令：
  - `list`：列出所有发现的 skills/prompts/themes，含来源范围（project/user）、路径、frontmatter 状态
  - `info <name>`：单个资源详情；未找到返回非零退出码
  - `doctor`：输出 ResourceDiagnostic 列表（重复名/缺 frontmatter/不可读文件）；有问题返回非零退出码，便于脚本/CI 集成
- 全部只读，不创建/修改/删除任何文件
- 复用 `DefaultResourceLoader` 已加载结果，不独立重扫，保证与运行时所见一致

## 明确不做
- 不做 `install` / `update` / `remove`（手动 `git clone` / `git pull` / `rm -rf` 即可）
- 不做 npm registry / semver / tarball 解包（Rust 不复用 TS 生态）
- 不做 extensions / themes 渲染（无扩展加载器、无 TUI）
- **不再依赖 c90**（砍 npm 后无需 config-value-resolver 的动态凭据）

## Capabilities
- resource-discovery

## Impact
- 非破坏性：新增只读 CLI 子命令，复用现有 ResourceLoader。
- change id 从 `c120-add-package-manager` 改为 `c120-add-resource-diagnostics`（更诚实反映内容）。
- capability 从 `package-management` 改为 `resource-discovery`。
- 去掉对 c90 的 depends_on；c90 本身独立保留（对 auth/凭据动态解析有价值，与 c120 无关）。
- 无新依赖。
