# 配置文件

`fontmin-rs build` 与 TypeScript package 支持相同的配置文件名。自动发现使用
以下精确顺序：

1. `fontmin.config.ts`
2. `fontmin.config.mts`
3. `fontmin.config.mjs`
4. `fontmin.config.cjs`
5. `fontmin.config.json`
6. `fontmin.config.jsonc`

运行 `fontmin-rs init` 可在当前目录创建初始 `fontmin.config.jsonc`。

JSON 和 JSONC 是 Rust CLI 无外部依赖的配置格式：CLI 完全在 Rust 中解析
它们，不会启动 Node.js。可执行 TS、MTS、MJS 和 CJS module config 需要
Node.js 22 或更新版本。

## JSONC 示例

```jsonc
{
  "input": ["fixtures/fonts/ttf/roboto-regular.ttf"],
  "outDir": "build",
  "clean": true,
  "subset": {
    "text": "Hello",
    "basicText": true,
    "keepLayout": "conservative",
  },
  "outputs": [{ "format": "woff2" }, { "format": "woff" }, { "format": "css" }],
  "css": {
    "fontFamily": "Roboto",
    "fontPath": "./",
    "fontDisplay": "swap",
  },
  "delivery": {
    "slices": [
      { "name": "latin", "unicodeRanges": ["U+0000-00FF"] },
      { "name": "cjk", "unicodeRanges": ["U+4E00-9FFF"] },
    ],
  },
  "cache": {
    "enabled": true,
    "dir": "node_modules/.cache/fontmin-rs",
  },
  "otf": {
    "variationCoordinates": { "wght": 700, "opsz": 14 },
  },
}
```

运行：

```sh
fontmin-rs build --config fontmin.config.jsonc
```

对于 SVG icon 集合，可以把输入、输出和 CSS 选项放在 JSONC 中，再通过命令行选择 iconfont preset：

```jsonc
{
  "input": ["icons/*.svg"],
  "outDir": "build/icons",
  "css": {
    "fontFamily": "Project Icons",
    "fontPath": "/icons",
  },
}
```

```sh
fontmin-rs build --config fontmin.config.jsonc --preset iconfont
```

## TypeScript 示例

```ts
import { modernWeb } from 'fontmin-rs'

export default async () => ({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  plugins: modernWeb({ text: 'Hello' }),
})
```

Module 可以通过默认导出或名为 `config` 的具名导出提供配置。导出值可以是
配置对象，也可以是返回配置对象的同步或异步函数。两种导出同时存在时，
优先使用默认导出。

Module config 是可执行的项目代码。Rust CLI 不会对其进行 sandbox；请只
运行受信任的配置。配置会继承 CLI 的环境和工作目录，因此普通 import 和
环境变量读取可以正常工作。

## Rust CLI Module 边界

Rust CLI 接受 JSON-compatible 配置数据，以及以下内置项的可序列化
descriptor：`glyph`、`unicodeSlices`、`otf2ttf`、`ttf2woff`、
`ttf2woff2`、`ttf2eot`、`ttf2svg`、`svg2ttf`、`svgs2ttf` 和 `css`。
`modernWeb()` 与 `fontminCompatPreset()` 返回的 descriptor 也受支持。

Rust CLI 不会执行自定义 JavaScript plugin hook。自定义 plugin 或 transform
函数、函数类型的 `css.fontFamily`、未知的内置 descriptor，以及 Rust
pipeline 无法表示的内置选项都会被拒绝。诊断中会包含最近的字段路径，例如
`plugins[1].transform`、`plugins[0].native.options.fallback` 或
`css.fontFamily`。这些限制适用于 Rust CLI bridge；Node pipeline 仍支持
自定义 JavaScript plugin。

## 配置目录与命令行覆盖

未设置 `cwd` 时，module 与 JSON/JSONC 配置都会将配置文件所在目录用作
`cwd`。相对的输入路径、`outDir`、缓存目录、`subset.textFile`，以及内置
`glyph` plugin 的 `textFile` 都从该目录解析；显式 `cwd` 会改变这个基准。
Rust CLI 会先求值并加载配置，再应用命令行中的输入、输出、subset、缓存、
preset、CSS、delivery 和 variation override。

加载并运行：

```ts
import { loadConfig, optimize } from 'fontmin-rs'

await optimize(await loadConfig())
```

## 关键字段

| 字段               | 说明                                                      |
| ------------------ | --------------------------------------------------------- |
| `cwd`              | 相对路径解析基准；未传时使用当前工作目录或配置文件目录    |
| `input`            | 输入文件列表；CLI 支持 glob 展开                          |
| `outDir`           | 输出目录                                                  |
| `clean`            | 构建前清空输出目录                                        |
| `preserveOriginal` | 是否保留原始输入资产                                      |
| `runtime`          | Node pipeline runtime：`native`（默认）、`wasm` 或 `auto` |
| `otf`              | OTF 转 TTF 选项，包括 CFF2 variation 坐标                 |
| `subset`           | 子集化选项                                                |
| `outputs`          | 带格式和可选文件名/扩展名的输出配置                       |
| `css`              | `@font-face` CSS 生成选项                                 |
| `delivery`         | 具名 Unicode 分片交付                                     |
| `cache`            | pipeline 缓存选项                                         |
| `plugins`          | Plugin 列表；Rust CLI 接受可序列化的内置 descriptor       |

## Node Pipeline Runtime

TypeScript `optimize()` pipeline 接受 `runtime: 'native' | 'wasm' | 'auto'`。`native` 是默认值；`wasm` 强制 pipeline 的所有内置操作使用随包发布的 WASM module；`auto` 为整个 pipeline 选择一个 runtime，并且只在 native binding 无法加载时回退到 WASM，转换错误永远不会触发回退。自定义 JavaScript plugin 和所有文件 I/O 始终在 Node 端运行。

当省略 `runtime` 时，`ttf2woff2()` 的旧 `fallback` 选项会作为 pipeline runtime。相同值允许共存，不同值会抛出冲突；多个 plugin 使用不同 fallback 也会冲突；`fallback: 'js'` 始终不受支持。完整矩阵见 [Node API](../api/node#pipeline-runtime)。

## 子集化选项

| 字段              | 说明                                                |
| ----------------- | --------------------------------------------------- |
| `text`            | 需要保留的文本                                      |
| `textFile`        | 从文件读取并追加的文本                              |
| `unicodes`        | 需要保留的 Unicode code points                      |
| `basicText`       | 保留基础文本字符集                                  |
| `preserveHinting` | 保留 hinting 信息                                   |
| `trim`            | 裁剪未使用字形；`false` 会在校验后保留原始 TTF 数据 |
| `keepNotdef`      | 保留 `.notdef` 字形                                 |
| `keepLayout`      | `drop`、`conservative` 或 `preserve`                |

## Unicode 分片交付

设置 `delivery.slices` 后，会在所选格式转换前为每个具名范围组生成一份子集。分片名必须唯一，且只能包含字母、数字、连字符或下划线。每个 `unicodeRanges` 条目都接受 `U+HEX` 或 `U+HEX-HEX` 形式；每个端点使用一到六位十六进制数字。

上面的示例会生成 `roboto-regular-latin.*` 和
`roboto-regular-cjk.*`。CSS 输出会为每个分片写入独立的 `unicode-range` 描述符，并优先于该来源的全局 CSS `unicodeRanges` 选项。

## CFF/CFF2 OTF 输入

Rust 构建引擎会在子集化与 Web 转换前，将受支持的 OTF 输入规范化为静态 TrueType。通过 `otf.variationCoordinates` 选择 CFF2 实例。重复的 `build --variation TAG=VALUE` 会覆盖该对象中同名轴的值，同时保留其他已配置轴。静态输出不会保留 CFF2 variation 表或 Type 2 hinting。

## CSS 选项

| 字段            | 说明                                                |
| --------------- | --------------------------------------------------- |
| `fontFamily`    | `@font-face` 的 `font-family`                       |
| `fontPath`      | CSS 中字体文件路径前缀                              |
| `fontDisplay`   | `font-display` 值                                   |
| `local`         | 是否生成 local source                               |
| `glyph`         | 生成 icon glyph class 规则                          |
| `iconPrefix`    | 生成 glyph class 时使用的 class 前缀                |
| `asFileName`    | 使用 SVG icon 文件名作为 class 后缀                 |
| `base64`        | 是否内联字体内容                                    |
| `target`        | CSS、SCSS 或 Less 输出目标                          |
| `unicodeRanges` | 当来源未定义范围时使用的全局 `unicode-range` 描述符 |
