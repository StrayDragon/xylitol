# Tasks: c1620-add-print-eval-foundations

## 1. Print trust 旗标

- [x] 1.1 修订 `cli-entry`：`ce19` 允许 `print` 接受 `--trust`/`--no-trust`；`tests/features/cli-entry.feature` 增加 print trust 场景；实现 `PrintSurfaceArgs` + bootstrap `trust_override`
- [x] 1.2 验证：`--trust` 在含 `.xylitol/` 的 cwd 下项目资源可加载路径可达（BDD 或 bootstrap 单测）；`--no-trust` 跳过项目资源

## 2. Print 失败非 0 exit

- [x] 2.1 修订 `cli-print`：流中 `XyEvent::Error` → `run_print`/`render_stream` 返回错误；`main` 非 0
- [x] 2.2 可执行场景或单测：注入 Error 事件 → 非 0；无 Error 正常结束 → 0

## 3. 可选 session.max_turns

- [x] 3.1 `runtime-config`：`SessionConfig.max_turns: Option<u32>`；`rc22` 澄清；非法 0 加载失败；单测
- [x] 3.2 `agent-runtime`：新 req——配置 `max_turns=N` 时装配 `should_stop_after_turn`；缺省不装；live feature 场景
- [x] 3.3 组合根接线：bootstrap/composition 读取配置并 `set_should_stop_after_turn`；`example.yaml` 注释与真实字段对齐

## 4. 收尾

- [x] 4.1 `llman sdd validate c1620-add-print-eval-foundations --strict --no-interactive`（结构）与相关 BDD/单测绿
- [x] 4.2 同步 roadmap「Print 完备度」表：trust / exit / max_turns 标为进行中或已提案
