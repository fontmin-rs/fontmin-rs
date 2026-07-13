# 配置文件

`fontmin-rs build` 与 TypeScript package 共用配置文件。发现顺序为 `fontmin.config.ts`、`.mts`、`.mjs`、`.cjs`、`.json`、`.jsonc`。

JSON/JSONC 由 Rust 直接解析，不依赖 Node.js。模块配置需要 Node.js 22 或更高版本，可通过 `default` 或 `config` 导出对象、同步工厂或异步工厂。模块配置属于可执行项目代码，只应运行可信配置。Rust CLI 接受可序列化的内置插件和 preset；自定义 JavaScript hook 与函数型 CSS family 仅能在 Node pipeline 中使用。

运行 `fontmin-rs init` 可在当前目录创建初始 `fontmin.config.jsonc`。

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
import { defineConfig, modernWeb } from 'fontmin-rs'

export default async () =>
  defineConfig({
    input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
    outDir: 'build',
    cache: { enabled: true },
    plugins: modernWeb({
      text: 'Hello',
      fontFamily: 'Roboto',
      fontPath: './',
    }),
  })
```

模块工厂也可以是同步函数；没有 default export 时，也可以使用具名 `config` export。

Rust CLI 可以直接执行同一模块：

```sh
fontmin-rs build --config fontmin.config.ts
```

加载并运行：

```ts
import { loadConfig, optimize } from 'fontmin-rs'

await optimize(await loadConfig())
```

## 关键字段

| 字段               | 说明                                                   |
| ------------------ | ------------------------------------------------------ |
| `cwd`              | 相对路径解析基准；未传时使用当前工作目录或配置文件目录 |
| `input`            | 输入文件列表；CLI 支持 glob 展开                       |
| `outDir`           | 输出目录                                               |
| `clean`            | 构建前清空输出目录                                     |
| `preserveOriginal` | 是否保留原始输入资产                                   |
| `otf`              | OTF 转 TTF 选项，包括 CFF2 variation 坐标              |
| `subset`           | 子集化选项                                             |
| `outputs`          | 带格式和可选文件名/扩展名的输出配置                    |
| `css`              | `@font-face` CSS 生成选项                              |
| `delivery`         | 具名 Unicode 分片交付                                  |
| `cache`            | native pipeline 缓存选项                               |
| `plugins`          | 内置插件描述符；自定义 JS hook 仅由 Node 支持          |
| `runtime`          | Node `optimize()` runtime：`native`、`wasm` 或 `auto`  |

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
