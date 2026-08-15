# 支持策略

本页定义 `main` 分支持续验证的运行环境。公开 API 清单与机器可读的
[`contracts/support.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/support.json)
共同构成稳定的 `1.1` 契约；发布前，这里列出的每个环境都必须通过相同的一致性、
兼容性与打包门禁。

## Node.js

| 入口                         | 支持版本                    | 验证证据                                      |
| ---------------------------- | --------------------------- | --------------------------------------------- |
| `fontmin-rs` CLI 与 Node API | Node.js 22.18、24、26       | Linux、macOS、Windows 完整测试矩阵            |
| Native binding packages      | 下列 target 上的 Node-API 8 | Native 构建与 tarball consumer smoke test     |
| `@fontmin-rs/wasm` 工具链    | Node.js 22.18 或更高        | Typecheck、构建、Vitest 与浏览器包 smoke test |

发布的 `fontmin-rs` 包声明 `node >=22.18.0`。更新的 Node.js major 在加入 CI 矩阵前
按 best-effort 支持。加载可执行的 TypeScript、MTS、MJS、CJS 配置也要求 Node.js
22.18 或更高版本。

## Native 平台

Release workflow 会构建并打包以下精确 target：

| 操作系统 | CPU   | Runtime               |
| -------- | ----- | --------------------- |
| macOS    | x64   | Darwin native binding |
| macOS    | arm64 | Darwin native binding |
| Windows  | x64   | MSVC native binding   |
| Windows  | arm64 | MSVC native binding   |
| Linux    | x64   | glibc                 |
| Linux    | x64   | musl                  |
| Linux    | arm64 | glibc                 |
| Linux    | arm64 | musl                  |

Node `runtime: "auto"` 会优先加载对应 native package，无法加载时回退到 WASM。
`runtime: "native"` 是硬性要求；加载失败会返回错误，不会静默切换行为。

## 浏览器 WASM

浏览器包会在当前 Playwright Chromium、Firefox 和 WebKit 中测试。其公开边界为异步、
纯内存 API，不支持文件路径、glob、磁盘缓存、CLI 或任意 Node.js plugin hook。

Native 与 WASM 对每个内置 transform、preset、输出 metadata 契约和 malformed input
诊断运行相同的语义一致性 corpus。字节级完全相同不属于兼容性承诺。

## Rust 工具链

- **MSRV：** Rust 1.88.0，由 workspace metadata 声明，并通过
  `cargo check --locked --workspace --all-targets --all-features` 验证。
- **固定开发与发布工具链：** Rust 1.97.1，用于格式化、Clippy、测试、覆盖率、
  native/WASM 构建和发布。
- **Fuzzing：** 当前 nightly，仅用于独立 cargo-fuzz workspace 和定时
  AddressSanitizer 任务。

每次准备发布或依赖要求升级时复核固定工具链。升级必须成为显式仓库提交并通过完整
release gate。1.0 前提高 MSRV 必须提供 Changelog 和迁移说明；1.0 后至少通过
minor 版本发布。

## 支持边界

只有本矩阵列出的环境会阻断发布。其他操作系统、CPU、Node.js 版本和浏览器可能通过
WASM 正常运行，但在进入 CI 前均按 best-effort 支持。报告问题时请附带 runtime、
target、输入格式和诊断输出。
