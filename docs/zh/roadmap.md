# 首个稳定版路线图

fontmin-rs `0.1.0-rc.1` 已发布 CLI、Node.js 包、浏览器 WASM 包、native
binding，以及 8 个平台包。当前公开能力已经足以进入真实项目；首个稳定版前的重点不再是快速扩大 API，而是证明这些契约足够稳定、正确，并且能够可靠发布和回滚。

当前发布线以 `0.1.0` 作为首个稳定版本，并不表示已经达到 SemVer
`1.0.0`；未来的 1.0 会运行独立的 RC 周期。

路线图采用“退出条件”，不承诺日历日期。只有当检查能够在 `main` 和 release
workflow 中稳定复现时，对应里程碑才算完成。

## 当前基线

- 发布门禁会核对 11 个 npm 包、Cargo metadata、运行时内嵌版本、Changelog 和 tag 是否使用同一版本。
- CI 覆盖格式化、Rust/TypeScript 零警告 lint、Node.js 22/24/26、WASM、浏览器加载、文档 Playground、native 包 smoke test、发布准备检查和 benchmark。
- 发布前会拒绝 high/critical 依赖安全问题，要求 Rust 行覆盖率不低于 80%，检查 npm tarball 内容，并运行消费者 smoke test。
- 共享字体 fixture 的清单和 checksum 由 `pnpm run fixtures:check` 校验。
- 本地开发与所有 GitHub workflow 使用同一个固定的 Rust 1.97.1 toolchain；升级必须通过显式仓库改动完成。
- [性能策略](./benchmarks.md)会在固定 CI 软件环境中构建 release profile binding，
  汇总三轮结果，并对配对兼容流水线设置门禁；绝对耗时保留用于诊断。
- 受许可证约束的 fixture corpus 已覆盖 Latin、紧凑 CJK、icon font、CFF、CFF2、
  variable font 和 malformed input，并记录可复现来源。
- Native 与 WASM 对所有内置 transform、preset、输出 metadata 和 malformed
  diagnostic 运行同一套语义一致性矩阵。
- 有时间上限的 AddressSanitizer cargo-fuzz 任务会在相关改动和每周定时任务中运行；
  最小化后的 crash 会成为永久 malformed fixture。
- Rust 1.88.0 是独立声明并由 CI 验证的 MSRV；固定工具链与升级节奏见
  [支持策略](./support.md)。
- Release profile 下的 `glyph + ttf2woff` 基线在记录机器上约比经典 Fontmin 快
  6.73 倍；此前 debug profile 的测量已经废弃。
- [弃用策略](./deprecation.md)、[故障排查](./troubleshooting.md)、
  [安全策略](https://github.com/fontmin-rs/fontmin-rs/security/policy)、迁移指南和发布回滚流程已经定义从预发布到 1.0 的维护路径。
- Rust advisory 检查不再接受例外；当前 npm audit 问题通过限定范围且经过 lockfile
  验证的 override 解决。

## Beta 加固——已完成

Beta.3 与 beta.4 已连续通过同一套完整发布门禁。Beta.4 直接从已经全绿的候选提交打
tag，发布期间没有修复 metadata、修改代码或回滚平台包。永久 malformed corpus 仍会
继续接收 fuzzing 和真实项目发现的最小化输入。

退出条件：连续两个 beta 版本完整通过 release gate，且不需要人工修复 metadata 或回滚平台包。

## 0.1 Release Candidate

RC 阶段冻结面向用户的契约，把重点转向兼容性证据。

- [机器可读公开契约](./contracts.md)已经冻结 CLI flags/exit codes、配置 schema、
  Node/WASM exports、plugin lifecycle、diagnostic codes 和生成文件命名规则。
- [支持矩阵](./support.md)已经发布 Node.js 版本、操作系统、CPU/libc target、
  浏览器 WASM 能力，以及 Rust library consumer MSRV。
- 对代表性的 Fontmin pipeline 比较 glyph coverage、可解析输出、CSS 语义和文件命名；不要求字节完全一致。
- 安装、CLI、ESM、浏览器、native、native fallback 和 forced-WASM 路径均从打包后的
  tarball 验证，而不是依赖 workspace import。

退出条件：冻结后的契约和支持矩阵经过一个 RC 周期，且不存在未解决的 P0/P1 正确性、安全或打包问题。

## 0.1.0 稳定版门禁

满足以下全部条件时才发布 `0.1.0`：

- 公开 API 和配置契约有完整文档与兼容性测试。
- 每条受支持的字体路径要么生成可解析且覆盖符合请求的输出，要么返回稳定、可操作的诊断；malformed input 不会跨公开边界触发 panic。
- Native packages 与 WASM fallback 在所有承诺支持的 target 上通过同一套 conformance corpus。
- Rust 行覆盖率保持不低于 80%，lint 零警告，package smoke test 通过，且没有被接受的 high/critical 依赖安全问题。
- Release profile 下，代表性兼容 pipeline 的性能至少与经典 Fontmin 持平；native subset 与 web-font conversion 保持在约定回退预算内。
- Release workflow 能从干净 tag 发布全部包、创建 GitHub Release，并校验 npm dist-tags，不依赖本地人工操作。

并非 `0.1.0` 必需的工作——例如覆盖所有历史 Fontmin plugins、所有字体格式边缘情况或分布式缓存——应明确放入 0.1 之后，而不是无限推迟首个稳定版。

## 迈向 1.0

`0.1.0` 发布后，真实项目兼容性证据、更多字体 fixture 和受维护的依赖替代方案将共同决定 1.0 范围。已经文档化的 RC 契约继续受弃用策略约束。未来发布
`1.0.0` 时会使用单独版本的 RC，而不会把 `0.1.0` 候选周期视为 1.0 验证。
