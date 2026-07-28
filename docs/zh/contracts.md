# 公开契约

稳定公开边界记录在
[`contracts/public-api.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/public-api.json)
中。CI 会将实现与这份机器可读清单比较。修改清单代表一次明确的兼容性决策，必须在同一个
PR 中同步更新 Changelog、迁移说明和相关测试。

为 `1.0` 重新确认的环境和 runtime 边界记录在
[`contracts/support.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/support.json)。
CI 会将该清单与 package engine、Cargo metadata、toolchain、native build target 和
浏览器矩阵比较。

## CLI

`build`、`subset`、`coverage`、`inspect`、`convert`、`bench`、`init` 和
`doctor` 命令，以及清单列出的长参数都被冻结。命令成功时退出码为 `0`；参数、配置、
I/O 或处理失败时退出码为 `1`。如果错误带有稳定 diagnostic code，脚本还应优先按
code 分支。

帮助信息的排版、空白、终端颜色和面向人的措辞不属于冻结范围；命令名、可接受参数、退出码
和 diagnostic code 属于冻结范围。

## 配置

Rust CLI、Node 包和浏览器 WASM 包有意采用不同边界：

- Rust 接受项目字段、可序列化的内置 plugin descriptor，以及文档列出的六种配置扩展名。
- Node 接受文件系统或内存输入、自定义 plugin hooks，以及
  `runtime: "native" | "wasm" | "auto"`。
- 浏览器 WASM 只接受内存中的 `assets` 和 `plugins`。

精确的顶层字段记录在契约清单中；选项行为和嵌套字段继续以[配置文档](./guide/config.md)
为准。

## JavaScript exports 与 plugin 生命周期

`fontmin-rs` 主入口、`./plugins`、`./presets`、`./compat` 子路径，以及
`@fontmin-rs/wasm` 的运行时导出名称被冻结。仅类型导出遵循相同兼容策略，并由
TypeScript 编译测试把关。

Node plugin 按 `buildStart`、`transform`、`generateBundle`、`buildEnd` 的顺序
执行。破坏性的 hook 签名或顺序变化必须遵循弃用策略，并进行明确的版本规划。

## Diagnostics 与文件命名

Rust 产生的稳定诊断使用清单列出的 `fontmin::*` code，包括
`fontmin::invalid_font`。Node 与 WASM 通过 `FontminDiagnosticError` 暴露这些
code。

生成文件遵循以下模板：

| 输出                        | 模板                         |
| --------------------------- | ---------------------------- |
| 普通 transform              | `{stem}.{extension}`         |
| 命名 Unicode delivery slice | `{stem}-{slice}.{extension}` |
| SVG icon font 默认 stem     | `iconfont`                   |
| 保留的原始输入              | `{input-file-name}`          |

显式的 `fileName`、`ext` 或 icon-font `fontName` 会覆盖对应默认值。测试冻结代表性
输出集合和 CSS URL，不承诺编码器输出字节完全一致。

## 兼容性规则

即使是新增能力，也必须更新清单并通过完整发布门禁。删除或修改现有项应遵循
[弃用策略](./deprecation.md)。`1.0.0` 会在稳定版提升前运行独立版本的 RC 周期，
并收集[独立兼容性证据](./compatibility.md)。
