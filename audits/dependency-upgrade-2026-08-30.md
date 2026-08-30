# 依赖升级与代码检测报告

- 检测日期：2026-08-30
- 仓库：`fontmin-rs`
- 结论：Rust、Node.js、文档站、WASM、N-API 与 vendored crates 的依赖已升级到当前清单和 Rust 1.98 可解析的最新版本；构建、格式化、静态分析、类型检查、测试、安全审计、文档构建和覆盖率门槛全部通过。
- 检测环境：macOS arm64、Node.js 24.19.0、pnpm 11.15.1、Rust 1.98.0。

## 升级范围

### Rust

工作区 MSRV 从 Rust 1.88.0 提升到 1.98.0，固定工具链从 1.97.1 提升到 1.98.0；CI 与发布工作流同步固定到 `dtolnay/rust-toolchain` 的 Rust 1.98.0 提交 `f8be11a05b1d4f3fcebe6410cc16743212b999b0`。

主要直接依赖升级如下：

| 依赖           | 升级前 | 升级后 |
| -------------- | ------ | ------ |
| `base64`       | 0.22   | 0.23   |
| `bpaf`         | 0.9.26 | 0.9.27 |
| `jsonc-parser` | 0.33.0 | 0.33.1 |
| `napi`         | 3.10.5 | 3.12.2 |
| `napi-build`   | 2.3.2  | 2.4.1  |
| `napi-derive`  | 3.5.10 | 3.6.3  |
| `skrifa`       | 0.44.0 | 0.46.2 |
| `wasmi`        | 0.46   | 1.1    |

同时刷新了根工作区、fuzz 及 vendored crates 的锁文件和直接依赖。代表性 vendored 升级包括 `brotli` 8.0.4、`bytes` 1.12.1、`thiserror` 2.0.20、`itertools` 0.15、`serde` 1.0.229、`serde_json` 1.0.151、`ttf-parser` 0.25.1。

`cargo upgrade --dry-run` 与 `cargo update --dry-run` 最终均无待升级项。

### Node.js

使用 pnpm workspace 递归升级并刷新 `pnpm-lock.yaml`。主要直接依赖升级如下：

| 范围              | 依赖              | 升级前         | 升级后         |
| ----------------- | ----------------- | -------------- | -------------- |
| workspace catalog | `@napi-rs/cli`    | 3.7.3          | 3.8.6          |
| workspace catalog | `@types/node`     | 26.1.1         | 26.4.0         |
| workspace catalog | `tsdown`          | 0.22.12        | 0.22.14        |
| workspace catalog | `vitest`          | 4.1.10         | 4.1.11         |
| root              | `bumpp`           | 11.1.0         | 12.2.2         |
| root              | `npm-run-all2`    | 9.0.2          | 9.0.3          |
| root              | `oxfmt`           | 0.59.0         | 0.65.0         |
| root              | `oxlint`          | 1.74.0         | 1.80.0         |
| package           | `fonteditor-core` | 2.4.1          | 2.6.3          |
| package           | `playwright`      | 1.61.1         | 1.62.1         |
| package           | `tinybench`       | 6.0.2          | 6.1.4          |
| docs              | `@vueuse/core`    | 14.3.0         | 14.4.0         |
| docs              | `tinysaver`       | 0.1.0          | 0.3.0          |
| docs              | `@vue/test-utils` | 2.4.11         | 2.5.0          |
| docs              | `happy-dom`       | 20.11.0        | 20.12.0        |
| docs              | `unocss`          | 66.7.5         | 66.8.1         |
| docs              | `vitepress`       | 2.0.0-alpha.18 | 2.0.0-alpha.19 |
| docs              | `vue`             | 3.5.40         | 3.5.42         |

`pnpm outdated --recursive` 最终只列出 7 个与当前 macOS arm64 不匹配的可选原生平台包；它们属于 Linux、Windows 或 macOS x64 发布目标，不是实际待升级依赖。

## 兼容性调整

- 适配 `wasmi` 1.1：使用 `instantiate_and_start` 替代旧的 `instantiate(...).start(...)` 链路。
- 适配 `fonteditor-core` 2.6：内部保留原生 TTF 编辑器类型，避免过早转换造成的 TypeScript 类型不兼容；公开 API 未改变。
- 适配 Rust 1.98 Clippy：使用 `as_chunks` 和 `Duration::from_mins` 等新建议写法。
- 适配 oxlint 1.80：显式关闭与仓库既有代码规范冲突的新默认规则 `no-multi-assign`、`one-var` 和 `node/no-top-level-await`；其余警告仍按 `--deny-warnings` 执行。
- 同步更新支持策略、依赖重复审计、发布策略、CI 工作流和 N-API 生成绑定。
- 删除 pnpm 11.15.1 已不再识别的 `packageManagerStrict` workspace 配置。

## 检测结果

| 检测项                        | 结果 | 摘要                                                              |
| ----------------------------- | ---- | ----------------------------------------------------------------- |
| 完整仓库检查 `pnpm run check` | 通过 | 格式、lint、类型、fixtures、依赖策略、测试与文档检查全部通过      |
| Rust 编译                     | 通过 | workspace 全 targets、全 features；vendored library targets 通过  |
| Rust Clippy                   | 通过 | workspace 全 targets、全 features；safer-bytes vendored 检查通过  |
| TypeScript 类型检查           | 通过 | `pnpm typecheck`                                                  |
| Node/WASM/Docs/工具测试       | 通过 | package 247、WASM 36、docs 41、工具脚本 106，共 430 项            |
| Rust 测试                     | 通过 | workspace 403 项；allsorts CFF 回归测试 5 项                      |
| 文档构建                      | 通过 | VitePress 2.0.0-alpha.19 构建成功                                 |
| 依赖策略审计                  | 通过 | 4 组已记录重复依赖、5 个 vendored patch                           |
| 安全审计                      | 通过 | `cargo deny` advisories/sources 通过；`pnpm audit` 无已知高危漏洞 |
| 覆盖率                        | 通过 | 行覆盖率 83.59%，高于 80% 门槛；region 覆盖率 82.88%              |
| 依赖新鲜度                    | 通过 | Cargo 无可升级项；pnpm 无实际可升级项                             |

## 风险与后续建议

1. **MSRV 变化**：最低 Rust 版本从 1.88 提升到 1.98，这是对下游构建环境有影响的兼容性变化，发布说明应明确标注。
2. **重复 Rust 依赖**：当前锁文件保留 4 组经策略审阅的重复版本：`hashbrown`、`miniz_oxide`、`syn`、`unicode-width`。它们不阻塞发布，但会增加少量编译与产物成本。
3. **vendored patch 清理条件已到达**：`oxifont-core` patch 的既定退出条件包含 Rust 1.98；建议另开专项，在不夹带本次全量升级的情况下验证能否移除 patch，并重新检查依赖图和字体回归。
4. **vendored 上游测试不完全自包含**：对 vendored crates 强行执行 `--all-targets` 会遇到上游未随 vendoring 带入的测试字体、辅助模块，以及 `woff2-patched` 示例仍引用原 crate 名的问题。库目标和仓库实际使用的 allsorts CFF 回归测试均已通过；若需要把 vendored 全目标纳入 CI，应补齐或排除这些上游测试资产。
5. **本机覆盖率工具链**：Homebrew Rust 1.98 缺少 `llvm-profdata`，首次覆盖率报告合并失败；切换到带 `llvm-tools-preview` 的 rustup 1.98.0 后完整通过。CI 已固定 Rust 1.98.0，并安装 `cargo-llvm-cov`。
6. **非阻塞工具提示**：tsdown 提示 TypeScript 7.0 API 仍处于实验状态；当前构建和类型检查均通过。全局 `wasm-pack` 0.12.1 也提示有 0.15.0 可用，但它不是仓库依赖，本次未修改开发机全局工具版本。

## 最终结论

本次升级后的依赖图、代码兼容性和质量门禁均满足仓库当前要求，可以进入代码审查。建议发布时将 Rust 1.98 MSRV 提升作为显著变更说明，并把 oxifont vendored patch 的退出评估作为后续独立任务处理。
