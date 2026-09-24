# UI 运行时选型：gpui 桌面 + TUI 双 Rust 面

> **性质**：跨 change 主题级耐久底稿。产品方向正文在 [`../roadmaps/Gpui桌面客户端.md`](../roadmaps/Gpui桌面客户端.md)；本文只管「为什么这么选、什么条件下翻案」。
> 方向于 2026-08-22 收敛；2026-09-24 把 Web（TS + Bun）、Flutter 裸 FFI、Flutter 代码生成（FRB / rinf）与 gpui 并进同一张平台/覆盖面表。结论仍是 gpui。

## 决策

1. 第二产品面 = **gpui 桌面（Linux / Wayland 先行）**，与 TUI 并列双 Rust 面。独立 package，钉 Zed 修订，不跟上游 `main`。
2. **Web 不是编码主面**。c2310（薄 TS 客户端）draft 已删；Cloud-Agent 控制台 roadmap 搁置；TS 类型导出链路（specta bindings / gen-sdk / pa-bind1 drift 闸）已于 2026-08 移除。协议类型只以 Rust `protocol` 为真源。Web 只在「多工作区远程管控」复活时当管控台，Bun 做工具链。
3. **Flutter 不是当前端**。若将来要做，形状是薄客户端（Dart 打四象限信封）。裸 FFI 或 FRB 只包端侧局部调用。rinf 不采用为端到 Host 的总线。
4. **wasm 的定位是未来插件沙箱（WIT 组件），不是 UI 运行时**。yew / dioxus / leptos 不进入候选。

## 优先级输入（用户给定）

性能占用 → **内存极致少** → 原生 computer use（Hyprland/Wayland）→ 插件形态（wasm/lua）需可容纳。

## 架构前提

Host/client 已分离：模型/会话/MCP/trust 在 host（serve 或同进程），面只是四象限信封 client。因此：

- 内存大头在 host；UI 只贡献增量。同机 RSS 未实测，下文不写内存数字，只写进程里多出来的运行时。
- computer use 的 OS 访问在 host 工具侧（wlroots/grim/wtype 类）。UI 只呈现进度和确认，选型比的是窗口质量与类型成本。
- UI 选型决定呈现质量、本机集成面（剪贴板/编辑器/通知/全局键），以及将来「面的扩展」归谁管。插件住 host，一套服务所有面。

## 形态

Flutter 的三种接法共享引擎，差别在边界。Bun 是 JS 工具链，不画像素。

| 候选 | 谁画像素 | 谁写界面 | 和 Rust 怎么说话 |
|---|---|---|---|
| **gpui** | GPU 直绘（Linux 上 wgpu + Wayland/X11） | Rust | 直接 `use` 协议闭集 |
| **Flutter 裸 FFI** | Flutter 引擎（Linux 3.47 起默认 Impeller） | Dart | `dart:ffi` 手写 C ABI，无代码生成 |
| **Flutter + FRB** | 同上 | Dart | 从 Rust API 生成 Dart 调用。函数形态，可双向、可 Stream |
| **Flutter + rinf** | 同上 | Dart | 从带 Serde 的 Rust 结构体生成信号。消息形态 |
| **Web（TS + Bun）** | 浏览器，或 Tauri 的系统 webview，或 Electron 自带 Chromium | TypeScript | HTTP/WS 解 JSON。Bun 负责安装、打包、dev server |

rinf 的公开设计是业务全在 Rust、Flutter 只画、两边走 Serde 信号。本仓库的业务已经在 Host。rinf 或 FRB 若被当成第二条事件总线，就和信封重复。Flutter 若要做端：Dart 自己打现有 POST + WebSocket。裸 FFI / FRB 只在少数端侧调用必须进 Rust 时才有意义（例如本地解码一块大二进制），不拿来搬运会话事件。

## 支持平台

| 候选 | 桌面 | 移动 | Web | 和本仓库方向的关系 |
|---|---|---|---|---|
| **gpui** | macOS（Metal）、Windows、Linux/FreeBSD（`wayland` 与/或 `x11` feature） | 无产品级移动端 | 上游有 `gpui_web` 目标，非本方向 | 产品只承诺 Linux / Wayland 先行。API pre-1.0，文档要求跟最新 stable Rust；本仓库钉 nightly，接的时候要单独看工具链是否打架 |
| **Flutter 裸 FFI** | Windows、macOS、Linux | Android、iOS | 不能加载 cdylib。Web 要另走 wasm/JS | 平台面等于 Flutter 原生端。Linux 桌面是 GTK 壳 + 自绘 |
| **FRB** | 生成物覆盖 Windows / Linux / macOS | Android、iOS | 官方覆盖 Web（JS 与 Wasm 两条） | Web 那条把 Rust 编成 wasm，和「端是信封客户端」无关 |
| **rinf** | 文档标明 Linux / Windows / macOS 已测 | Android、iOS 已测；8.10.0 加了 OpenHarmony | 文档标明 Web 已测；eLinux 实验 | 平台声明最满。维护面是个人仓库量级（约 2.7k star，v8.10.0，2026-03） |
| **浏览器页 + Bun** | 有浏览器即可 | 手机浏览器可开，不是原生壳 | 就是 Web | 边际成本最低：引擎已在用户的浏览器里 |
| **Tauri / Electron 壳** | 两者都覆盖三大桌面；Tauri 2 另有移动壳 | Tauri 2 可做 Android/iOS | 壳里的页面仍是 Web | Linux 上 Tauri 再起一份 WebKitGTK，Electron 再起一份 Chromium。和「浏览器里多一个标签」不是同一笔内存 |

Wayland 成熟度，2026-09 两边都未到装上就稳：gpui 仍在补负载下闪烁、全屏帧回调冻结、快速开关窗时的渲染器 panic；Flutter Linux 在 3.38+ 的 Wayland 缩放过有 VRAM 上涨（flutter#182192，后续有修复）。这不构成改选。

## 通用性覆盖面

通用性指：离开本仓库还能覆盖多少普通 GUI 需求，以及第三方能不能接。

| 面 | gpui | Flutter（三种接法共享引擎） | Web + TS |
|---|---|---|---|
| 控件与动画 | 窄。布局、文本、动作为 Zed 这种编辑器长出来的 | 宽。桌面、移动、动画、主题、无障碍树都有现成控件 | 最宽 |
| 长 transcript / 流式字 | 强。文本系统是编辑器级 | 中。虚拟列表够用；编辑器级文本要另找包 | 强 |
| diff / 代码预览 | 中。无现成 Monaco，有 Zed 的文本底子，要自建 | 弱。几乎从零，或内嵌 webview（则白付 Flutter 引擎） | 强。Monaco / CodeMirror 现成。这是 Web 的硬优势 |
| 键位能否和 TUI 同源 | 能。窗口吃下全部键 | 能 | 不能当主面。浏览器或 webview 先吃掉关闭、查找、缩放 |
| 剪贴板 / 通知 / 外部编辑器 | gpui 平台服务够得到 | Flutter 插件够得到 | 浏览器权限别扭；壳应用可以，又回到 webview 内存 |
| 第三方消费者 | 只有 Rust 嵌入方。协议类型零翻译 | Dart 要一份生成或手写的视图模型 | TS 要重立类型层。监听器上的 OpenAPI 只是人手调试，不是客户端生成真源 |
| 单人维护面 | 一门语言，一份重依赖，跟 Zed 修订 | Flutter SDK + Rust。裸 FFI 无生成器；FRB 生成器社区大；rinf 生成器小 | Bun/Node + TS。生态最大，离 Rust 真源最远 |

三种 Flutter 接法：

| | 裸 FFI | FRB | rinf |
|---|---|---|---|
| 生成 | 无。ABI 自己守 | 按 Rust 函数生成 Dart | 按 Serde 结构体生成信号 |
| 适合搬什么 | 几块字节、一个符号 | 一批调用、Stream、不要求 Serde 的类型 | 一批可序列化消息 |
| 拿来搬会话事件 | 手写解码，和信封重复 | 每个协议变更都进生成物 | 信号流和 WebSocket 事件流双份 |
| 对本仓库的位置 | 端侧偶发本地调用可以 | 同上，且生成器比 rinf 经用 | 不采用为端到 Host 的总线 |

## 和已落地拓扑怎么接

| 接法 | 进程里有什么 | 结论 |
|---|---|---|
| gpui 薄客户端，attach 本机 Host | GPU 窗口 + 协议类型 | 与产品 TUI 同一拓扑。编码主面 |
| Flutter 薄客户端，Dart 打信封 | Flutter 引擎 + HTTP/WS | 多一门语言。仅当要移动窄端或控件速度时 |
| rinf/FRB 同进程嵌整份 Host | 引擎 + 操作器 | 把「端是 client」收成同进程巨石。不采用 |
| 浏览器标签 | 用户已打开的浏览器里多一页 | 搁置的管控台（一窗多工作区、diff）用这条。Bun 做工具链 |
| 桌面 webview 壳 | 再一份 WebKit 或 Chromium | 键位问题还在，Linux 内存比标签差。不作为编码主面，也不优先于浏览器标签做管控台 |

## 插件形态

- mlua：~1MB 级依赖、嵌入浅、可 sandbox、冷启快。个人 agent hook / 工具脚本用这条。
- wasmtime + WIT 组件：隔离最强，依赖与内存重一档。
- 插件住 host，与 UI 选型正交。浏览器或 Flutter 端会诱导第二套 UI 扩展体系，所以编码主面保持 native-first。未立项前不预挖。

## 翻案顺序

满足对应条件才离开 gpui，按这个顺序，不跳步：

1. **维持 gpui**（现状）。独立 package，钉 Zed 修订。
2. gpui 在 Linux/Wayland 的维护成本失控 → **egui / iced / slint**，仍直接 `use` 协议类型。
3. 需要移动窄端或设置类界面速度 → **Flutter 薄客户端**。事件走信封；裸 FFI 或 FRB 只包端侧局部调用；不用 rinf 做总线。
4. 出现真实非 Rust 消费者，或多工作区远程管控变成日常 → **TS 页做管控台**，Bun 做工具链，重立类型层（c2310 思路可复刻）。键位冲突决定它不是编码主面。Cloud-Agent 底稿届时复活。

## 来源

- rinf 设计与平台：[Introduction](https://rinf.cunarist.org/introduction/)；Serde 信号：仓库 `documentation/source/upgrading.md`（7 → 8）；v8.10.0（2026-03-09）。
- FRB 平台（Android / iOS / Windows / Linux / macOS / Web）：[fzyzcjy/flutter_rust_bridge](https://github.com/fzyzcjy/flutter_rust_bridge)。
- gpui 仍 pre-1.0，Linux 用 `gpui_platform` 的 `wayland` / `x11`：Zed `crates/gpui/README.md`。
- Flutter Linux 默认 Impeller：Flutter 3.47 文档 `docs.flutter.dev/perf/impeller`。
- 本仓库信封与「OpenAPI 不是客户端生成真源」：`docs/architecture/远程体验与线协议.md`。

## 相关落点

- 方向正文：`docs/roadmaps/Gpui桌面客户端.md`
- 同源约束板：`docs/roadmaps/跨端同源.md`
- 搁置愿景：`docs/roadmaps/Cloud-Agent与Web控制台.md`
