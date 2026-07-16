# 功能概览

fontmin-rs 目前处于 beta 阶段。本页集中记录已经实现的公开能力，以及选择不同入口时需要了解的边界。第一次使用时，可以继续阅读[快速开始](./getting-started)。

## 使用入口

| 入口                  | 适用场景                 | Runtime 模型                                   |
| --------------------- | ------------------------ | ---------------------------------------------- |
| `fontmin-rs` CLI      | Shell 脚本、CI、单次处理 | 原生 Rust 可执行文件                           |
| `fontmin-rs` Node API | 构建集成和自定义流水线   | Native binding、强制 WASM 或 native-first auto |
| `@fontmin-rs/wasm`    | 浏览器和 Worker 应用     | 异步、纯内存 WASM                              |

npm 包还提供同名 CLI、类型化配置 helpers、内置插件声明、presets，以及
Fontmin-compatible chain。

## 字体处理能力

| 能力          | 支持的操作                                                                          |
| ------------- | ----------------------------------------------------------------------------------- |
| 字体子集化    | 按文本、文本文件、Unicode code point、basic text 或 Unicode range 分组保留字形。    |
| Web 字体转换  | TTF 与 WOFF、WOFF2 互转，并支持 WOFF metadata 和 private data block。               |
| 遗留格式转换  | TTF 与 EOT 互转、TTF 转 SVG font、SVG font 转 TTF。                                 |
| OpenType 转换 | 将静态 CFF OTF 或 CFF2 variable OTF 实例化为静态 TrueType `glyf` 字体。             |
| Icon font     | 将多个 SVG icon 合并为 TTF，并按需生成 glyph class CSS。                            |
| CSS 生成      | 生成 `@font-face` CSS、SCSS 或 Less，支持 local source、Base64 和 `unicode-range`。 |
| 字体信息检查  | 检测并读取 TTF、OTF、WOFF、WOFF2 和 EOT metadata。                                  |

Node 与浏览器的低层 API 可以直接调用这些操作；文件流水线则通过内置插件组合相同能力。完整可调用接口请查看 [Node API](../api/node) 和[浏览器 WASM API](../api/wasm)。

## 流水线与 Preset

- `optimize(config)` 在 Node.js 中发现文件输入、执行插件、复用匹配的缓存并写出产物。
- `optimizeBrowser(config)` 对命名的内存资产执行内置或自定义浏览器插件，不访问文件系统。
- `modernWeb(options)` 会规范化受支持的 OTF 输入、提取子集，并输出 WOFF、WOFF2 和 CSS。
- `fontminCompatPreset(options)` 按经典 Fontmin 顺序输出 EOT、SVG、WOFF、WOFF2 和 CSS。
- CLI `iconfont` preset 会将多个 SVG icon 合并为 `iconfont.ttf` 和 `iconfont.css`。
- `deliverySlices()` 与 CLI delivery slices 会生成命名子集及对应的 CSS `unicode-range` 描述。

内置插件 factories 包括 `glyph`、`deliverySlices`、`otf2ttf`、`ttf2woff`、
`ttf2woff2`、`ttf2eot`、`ttf2svg`、`svg2ttf`、`svgs2ttf` 和 `css`。

Node `optimize(config)` 会为整条流水线选择同一个内置 runtime：`native`、`wasm` 或 native-first `auto`。缓存会按所选 runtime 隔离；文件系统操作和自定义 JavaScript hooks 始终留在 Node.js 中执行。

## CLI 与配置文件

CLI 提供 `init`、`subset`、`convert`、`build`、`bench`、`inspect` 和
`doctor` 命令。它可以加载 JSON、JSONC、TS、MTS、MJS 和 CJS 配置文件；可执行的 module config 需要 Node.js 22 或更高版本。

命令参数和示例请查看[命令行](./cli)。配置文件发现、输出控制、缓存、子集化模式、OTF variation coordinates，以及 Rust built-ins 与自定义 Node.js plugins 的边界，请查看[配置文件](./config)。

## 兼容性与限制

- EOT 仅用于旧版 Internet Explorer 兼容；现代项目通常应首选 WOFF2，并用 WOFF 作为 fallback。
- CFF/CFF2 转换始终输出静态 TTF；variation tables 会被移除，Type 2 hinting 不会保留。
- `modernWeb()` 只输出 WOFF、WOFF2 和 CSS。需要遗留格式时应显式添加插件，或使用 `fontminCompatPreset()`。
- 浏览器包不支持路径输入、glob、CLI、磁盘缓存、输出目录或文件系统 hooks。
- 自定义 Node.js plugins 可以使用文件系统和 diagnostics context helpers；浏览器 plugins 只有更小的纯内存 hook surface。

Fontmin 用户可以阅读[从 Fontmin 迁移](./migration)。内部包结构和 runtime 边界请查看[项目架构](../architecture)。

## 项目状态

当前开发重点是发布加固、改进 diagnostics、性能优化，以及覆盖更多 Fontmin 兼容能力。长期方向记录在[设计文档](../../fontmin-rs-design)中；维护者可以使用[发布准备清单](../../releasing)检查候选版本是否达到发布门槛。
