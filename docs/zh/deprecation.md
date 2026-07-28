# 弃用策略

本策略适用于 CLI、Node/WASM API、配置文件、诊断码、生成文件命名和受支持的 runtime
行为。

## 1.0 就绪审计

没有 API 符合 `1.0` 的移除条件。`0.x` 发布线没有任何 API 完成一次完整的弃用窗口，
因此 `1.0` 会保留 `contracts/public-api.json` 中的全部公开项。

Fontmin-compatible 默认 export、作为 `preserveHinting` alias 的
`glyph({ hinting })`，以及 `ttf2woff2({ fallback })` runtime 选择仍是兼容路径。
它们不会被移除或静默改变。未来的弃用必须从下文的 replacement 与 warning 流程开始。

机器可读的决策记录在
[`contracts/support.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/support.json)。

## 1.0 之前

即使 SemVer 允许 `0.x` minor 版本包含破坏性变更，稳定的 `0.x` 公开契约仍受本策略
约束。每项破坏性变更都必须：

- 在 `CHANGELOG.md` 的 `Changed` 或 `Removed` 中明确记录；
- 在对应指南中给出可执行的迁移步骤；
- 在不引入正确性或安全风险时保留别名或兼容路径。

稳定的 `0.1` 契约已经冻结。破坏性变更必须进入新的 `0.x` minor 发布线，运行独立的
预发布验证周期，不能通过 patch 版本发布。

## 1.0 之后

Patch 版本不会移除公开行为。计划中的移除遵循以下顺序：

1. 引入替代方案并完成文档。
2. 在类型和 API 文档中标记旧行为为 deprecated。
3. CLI 或配置加载器能够可靠识别旧用法时，输出非致命警告。
4. 新旧路径至少共同保留一个 minor 版本。
5. 仅在 SemVer major 版本中移除旧路径。

安全或数据损坏修复可以跳过兼容期，但必须提供安全公告或醒目的 Changelog 原因，并在
可行时提供迁移路径。

## 诊断

弃用警告写入 stderr，不得污染 JSON 或二进制 stdout。Library 调用不会隐式打印警告；
通知应通过 TypeScript 标注、文档或返回的 diagnostic 传递。正式移除前，替代路径和
兼容路径都必须有测试覆盖。
