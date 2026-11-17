# Design — TUI 补全单栈收敛

## 现状（代码事实，2026-11-17 实测）

- 旧栈：AutocompleteProvider trait（autocomplete.rs:27）、CombinedAutocompleteProvider（:59，fd 评分 :297-332）、DebouncedAutocomplete（:365）。全仓非测试消费者 0（rg 实测：仅 src/lib.rs re-export 与 tests/autocomplete_test.rs）。
- 新栈：CompletionSource（completion.rs:31）3 实现 + CompletionRegistry 承载全部生产消费；fd 公共函数 fd_path_suggestions（completion.rs:381，c3145 落点）。
- 词汇表与帮助函数住旧栈文件，被 completion.rs:2-3、editor/autocomplete.rs、app-tui 两 source、demo 反向 import。
- 两份 feature 均裸规则（无 BDD 场景绑定，无 scenario binding 需迁移）；r1608 钉住的 set_autocomplete_provider 全仓代码 0 命中。
- 流程例外（crates/xylitol-tui/llmanspec/AGENTS.md）：行为合约 SSOT 在包实例 specs/；根 change 标 needs_specs_change: false；包实例 validate 用 --check（--strict 有工具假阳性）；归档门禁是根实例 validate --strict + review。

## 决策

### D1 词汇表落点：迁入 completion.rs（备选：独立 vocab 模块，否）

消费者全部在补全语境（CompletionSource 实现、editor、demo、app-tui sources）；独立模块只多一处跳转。lib.rs re-export 公开名单保持不变（API 边界 SSOT 不收窄；收窄如需要另行决策）。

### D2 capability 划界：两 capability 保留并重新划界（备选：物理合并为一，否）

- package-tui-autocomplete = 补全 fd 检索与取消（包级公共 API：walk_directory_with_fd / build_fd_path_query / fd_path_suggestions 对 editor 与非 editor 消费者开放）。
- package-tui-editor-autocomplete = editor popup 注册表与交互。
- 边界判据：fd 检索是 source 侧检索能力（包级 API 契约），popup 是 editor 交互行为；合并会把包级 API 契约塞进 editor 交互 capability。

### 条款处置

| req | 处置 | 理由 |
|---|---|---|
| r1578 | 删除 | 主语 CombinedAutocompleteProvider 删除；活语义（CancellationToken 取消 + SIGKILL）并入 r1579 |
| r1579 | 改述保留 | 行为真身 walk_directory_with_fd 活；主语去类型化、并入取消契约；「read_dir stub」改行为语「回退单层目录扫描」；情态词随触清用「必须」 |
| r1580 | 删除 | DebouncedAutocomplete 删除；editor 请求串行化与防抖由 editor-autocomplete r1611 承载 |
| r1608 shim 句 | 删句 | set_autocomplete_provider 代码 0 命中；其余条款不动、保持文件现状用词 |

## 风险

- completion.rs 迁入后约 +90 行（词汇表 + 帮助函数），文件变大但语义仍单一（补全）；以 just complexity 复杂度雷达关注，不以行数为约束。
- editor/autocomplete.rs 与包内测试 import 更新为机械改动，行为零变化。
