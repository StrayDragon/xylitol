# Tasks：c2540 簇头文件分句统一计数式

- [x] t1 specs：att24 文件分句句改（count-only，rules_edit_acked 已声明）+ att24 可执行场景措辞随动；`llman sdd validate` 结构绿
- [x] t2 实现：`file_clause` 纯计数式（删 basename 分支与 `basename()` 助手）；summary.rs 单测随动（单数 `1 file` / 复数 `N files` / 无路径检索裸动词不回归）
- [x] t3 BDD 随动：att24 场景断言（`Explored 1 file` / `Editing 1 file`）与 c2510 att35 单文件帧断言（`Exploring 1 file · 1 read`）转绿
- [x] t4 门禁收口：`just fmt` / `just lint` / `just test`；`llman sdd validate --strict`；review
