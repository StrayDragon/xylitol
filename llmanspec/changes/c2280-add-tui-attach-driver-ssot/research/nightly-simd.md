# nightly：默认别切，只有这些才值得单独开

本仓钉的是 **Rust 1.97.1 stable**（`rust-toolchain.toml`）。你说可以考虑 nightly。下面按「编 xylitol 本体要不要 nightly」分，避免把整条主线绑上 nightly。

tokio 继续用。io_uring、SIMD 是另开的门，不是换掉 tokio。

## 不用 nightly 就能做的（先做这些）

| 想加快的 | 用什么 | 工具链 |
|---|---|---|
| 消息别再用 JSON 文本 | `postcard`（还是 serde，只是包装换成二进制） | stable |
| TUI 读一长串历史少分配 | `rkyv` | stable |
| Linux 上文件/网络少进内核 | ntex `neon-uring`，或以后再看 `tokio-uring` | stable；要新内核（官方写 5.10+） |
| REST 调试口的 JSON | `sonic-rs`（Poem 有开关；我们自己 parse 也能调） | stable |

主仓 agent、TUI、测试继续 tokio + 1.97.1。

## 真要 nightly 的（单独 spike，别污染 `rust-toolchain.toml`）

| feature / 工具 | 干什么 | 和我们的关系 |
|---|---|---|
| `portable_simd`（`std::simd`） | 手写 SIMD 编解码 | 只有当你已经做了「专用 delta 格式」并且 CPU 剖面显示编解码占热，才值得。Gigatoken 整 crate 强制这个，已经被本仓 c1530 推迟 |
| Cranelift codegen | **编译变快**，不是跑 agent 变快 | 本地 `just` 开发可选用；CI 仍要 LLVM stable。见 skill `rust-build-tune` |
| `#[feature(...)]` 实验 API | 各式各样 | 没有一项是 attach / 吞吐的前提 |

切 nightly 的代价：CI 矩阵翻倍、依赖更容易跟 nightly 日期绑死、和 1.97.1 钉版本打架。

做法：主线稳定。需要 SIMD 或 Cranelift 时开 **独立 crate 或 `--config` 的 nightly job**，不要改根上 `channel`。

## 和 wire protocol 无关的

io_uring 帮的是「很多连接或很多磁盘读写」时少做 syscall。我们 TUI 到 server 通常就一两条连接。sandbox 里工具狂读仓库，才可能在 **server 进程** 上看到 uring 的好处。那也还是 stable 的 ntex/tokio-uring，不是 `portable_simd`。
