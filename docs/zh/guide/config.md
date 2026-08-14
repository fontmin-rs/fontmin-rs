# 配置文件

`fontmin-rs build` 与 TypeScript package 会发现相同的配置文件名，但两套 schema
包含少量 runtime 专属字段。自动发现使用以下精确顺序：

1. `fontmin.config.ts`
2. `fontmin.config.mts`
3. `fontmin.config.mjs`
4. `fontmin.config.cjs`
5. `fontmin.config.json`
6. `fontmin.config.jsonc`

运行 `fontmin-rs init` 可在当前目录创建初始 `fontmin.config.jsonc`。

JSON 和 JSONC 是 Rust CLI 无外部依赖的配置格式：CLI 完全在 Rust 中解析
它们，不会启动 Node.js。可执行 TS、MTS、MJS 和 CJS module config 需要
Node.js 22.18 或更新版本。

## JSON Schema

`fontmin-rs` npm 包内置 `configuration_schema.json`。在 JSON 或 JSONC
配置中加入它的本地路径，即可获得编辑器校验、自动补全和字段说明：

```jsonc
{
  "$schema": "./node_modules/fontmin-rs/configuration_schema.json",
  "input": ["fonts/*.ttf"],
  "outDir": "build",
}
```

当前项目已本地安装该包时，`fontmin-rs init` 会自动加入这项。通过临时安装
运行命令等导致本地 Schema 文件不存在时，初始化命令会省略它，避免生成
无法解析的悬空路径。

Schema 描述 npm CLI 与 Node loader 接受的 JSON 可序列化项目配置，其中也
包含 Rust CLI 共享字段；runtime 专属字段会在说明中标注。可执行 module
config 还可以使用内存输入和自定义 JavaScript plugin hook，这些文件应通过
`defineConfig()` 获得类型检查和补全。

## Rust CLI JSONC 示例

```jsonc
{
  "$schema": "./node_modules/fontmin-rs/configuration_schema.json",
  "input": ["fixtures/fonts/ttf/roboto-regular.ttf"],
  "outDir": "build",
  "clean": true,
  "subset": {
    "text": "Hello",
    "basicText": true,
    "keepLayout": "conservative",
    "missingGlyphs": "error",
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

如果希望根据语言覆盖范围和实际编码后的字节数自动规划边界，可用
`autoDelivery` 替换 `delivery`：

```jsonc
{
  "autoDelivery": {
    "frequencyText": "AB中文",
    "languages": ["en", "zh-Hans"],
    "targetBytes": 102400,
    "tolerance": 0.15,
    "maxSlices": 16,
    "measureFormat": "woff2",
  },
}
```

规划器会优先放置 `frequencyText` 中重复出现的字符、拆分过大分组、合并相邻的
过小分组，并让同一次 pipeline 执行中的所有 TTF face 共享计划，同时逐个验证编码体积。
省略 `languages` 时会从 `frequencyText` 检测语言预设。手工 `delivery` 与
`autoDelivery` 不能同时使用。

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

export default defineConfig({
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
descriptor：`glyph`、`unicodeSlices`（由 `deliverySlices()` 创建）、
`otf2ttf`、`ttf2woff`、
`ttf2woff2`、`ttf2eot`、`ttf2svg`、`svg2ttf`、`svgs2ttf` 和 `css`。
只要选项保持在这个可序列化内置边界内，`modernWeb()` 与
`fontminCompatPreset()` 返回的 descriptor 也受支持。

Rust CLI 不会执行自定义 JavaScript plugin hook。自定义 plugin 或 transform
函数、函数类型的 `css.fontFamily`、未知的内置 descriptor，以及 Rust
pipeline 无法表示的内置选项都会被拒绝。诊断中会包含最近的字段路径，例如
`plugins[1].transform`、`plugins[0].native.options.fallback` 或
`css.fontFamily`。因此，WOFF2 `fallback` 等仅用于 runtime 的 preset 字段会
被 Rust CLI 拒绝。这些限制适用于 Rust CLI bridge；Node pipeline 仍支持自定义
JavaScript plugin。

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

## 配置模型

Rust CLI 与 Node 包共享下列面向项目的基础字段。浏览器包不会加载项目配置文件，
而是直接接收纯内存的 `BrowserOptimizeConfig`。

| 字段               | Rust CLI | Node | 说明                                              |
| ------------------ | :------: | :--: | ------------------------------------------------- |
| `cwd`              |    ✓     |  ✓   | 相对路径基准；配置加载器默认使用配置文件目录      |
| `input`            |    ✓     |  ✓   | 路径与 glob；Node 还接受内存 `Uint8Array`         |
| `outDir`           |    ✓     |  ✓   | 输出目录                                          |
| `clean`            |    ✓     |  ✓   | 构建前清空输出目录                                |
| `preserveOriginal` |    ✓     |  ✓   | 兼容字段；当前由 outputs 控制产物保留             |
| `subset`           |    ✓     |  ✓   | 子集化选项；runtime 差异见下表                    |
| `autoDelivery`     |    ✓     |  ✓   | 按语言与实测 TTF/WOFF/WOFF2 字节数自动分片        |
| `outputs`          |    ✓     |  ✓   | 输出格式及可选文件名或扩展名覆盖                  |
| `css`              |    ✓     |  ✓   | `@font-face` CSS 生成选项                         |
| `cache`            |    ✓     |  ✓   | 缓存选项；Node 还接受 boolean                     |
| `plugins`          |    ✓     |  ✓   | Node 接受自定义 hook；Rust 只接受可序列化内置项   |
| `otf`              |    ✓     |  —   | Rust OTF-to-TTF 选项与 CFF2 variation 坐标        |
| `delivery`         |    ✓     |  —   | Rust 具名 Unicode 分片                            |
| `runtime`          |    —     |  ✓   | Node 内置操作 runtime：`native`、`wasm` 或 `auto` |

在 Node 中，应把 OTF 选项传给 `otf2ttf()` 或 `modernWeb()`，并通过
`deliverySlices()` plugin 添加手工 Unicode 分片。自动分片既可使用顶层
`autoDelivery`，也可使用 `autoDeliverySlices()` plugin；`otf` 与手工
`delivery` 仍是 Rust 专用顶层字段。

Rust schema 仍将 `parallel` 保留为预留字段。对于缺失字形检查，
`diagnostics.level` 控制是否打印 `warn` 消息，`diagnostics.failOnWarning`
会把覆盖率不完整的警告提升为错误；`diagnostics.pretty` 仍为预留字段。

## Node Pipeline Runtime

TypeScript `optimize()` pipeline 接受 `runtime: 'native' | 'wasm' | 'auto'`。`native` 是默认值；`wasm` 强制 pipeline 的所有内置操作使用随包发布的 WASM module；`auto` 为整个 pipeline 选择一个 runtime，并且只在 native binding 无法加载时回退到 WASM，转换错误永远不会触发回退。自定义 JavaScript plugin 和所有文件 I/O 始终在 Node 端运行。

当省略 `runtime` 时，`ttf2woff2()` 的旧 `fallback` 选项会作为 pipeline runtime。相同值允许共存，不同值会抛出冲突；多个 plugin 使用不同 fallback 也会冲突；`fallback: 'js'` 始终不受支持。完整矩阵见 [Node API](../api/node#pipeline-runtime)。

## 子集化选项

| 字段                | Rust | Node | 说明                                                   |
| ------------------- | :--: | :--: | ------------------------------------------------------ |
| `text`              |  ✓   |  ✓   | 需要保留的文本                                         |
| `textFile`          |  ✓   |  ✓   | 从文件读取并追加的文本                                 |
| `unicodes`          |  ✓   |  ✓   | 需要保留的 Unicode code points                         |
| `gids`              |  ✓   |  ✓   | 在 Unicode 选择之外额外保留的原始 glyph ID             |
| `glyphNames`        |  ✓   |  ✓   | 需要保留的精确 PostScript glyph 名                     |
| `unicodeRanges`     |  —   |  ✓   | 加入 Node 顶层 subset 的 Unicode 范围                  |
| `basicText`         |  ✓   |  ✓   | 保留基础文本字符集                                     |
| `preserveHinting`   |  ✓   |  ✓   | 裁剪时保留 `cvt `、`fpgm` 和 `prep`                    |
| `trim`              |  ✓   |  ✓   | 裁剪未使用字形；`false` 会在校验后保留原始 TTF 数据    |
| `keepNotdef`        |  ✓   |  ✓   | 保留 glyph zero 的原始轮廓                             |
| `retainGids`        |  ✓   |  ✓   | 保留原始 glyph ID，并留下空的中间槽位                  |
| `retainGlyphNames`  |  ✓   |  ✓   | 重写 `post` v2 并保留 PostScript glyph 名              |
| `retainLegacyCmap`  |  ✓   |  ✓   | 重映射并保留非 Unicode、非 symbol 的 `cmap` 记录       |
| `retainSymbolCmap`  |  ✓   |  ✓   | 重映射并保留 Windows symbol `cmap` 记录                |
| `keepLayout`        |  ✓   |  ✓   | 丢弃、重映射或严格拒绝已知 contextual layout 丢失      |
| `layoutFeatures`    |  ✓   |  ✓   | GSUB/GPOS feature tag 白名单；空数组表示全部保留       |
| `layoutScripts`     |  ✓   |  ✓   | GSUB/GPOS script tag 白名单；空数组表示全部保留        |
| `layoutLanguages`   |  ✓   |  ✓   | LangSys 白名单；`default` 表示各 script 的默认 LangSys |
| `nameIds`           |  ✓   |  ✓   | OpenType name ID 白名单；空数组表示保留全部记录        |
| `nameLanguages`     |  ✓   |  ✓   | 与 platform 相关的数值 name language ID 白名单         |
| `dropTables`        |  ✓   |  ✓   | 重写后需要移除的四字节可选表 tag                       |
| `passThroughTables` |  ✓   |  ✓   | 需要从源字体原样复制的四字节表 tag                     |
| `missingGlyphs`     |  ✓   |  ✓   | 缺失请求字形时使用 `ignore`、`warn`（默认）或 `error`  |
| `hinting`           |  —   |  ✓   | `preserveHinting` 的 Fontmin-compatible alias          |
| `clone`             |  —   |  ✓   | Node glyph plugin 运行时保留转换前资产                 |

Rust 顶层 `subset` 模型没有 `unicodeRanges` 字段。需要按范围生成独立产物时使用
`delivery.slices`；在受信任的 module config 中也可使用可序列化的
`glyph({ unicodeRanges })` descriptor。

`warn` 会报告缺失码点后继续生成，`error` 会在写出产物前停止，`ignore` 会跳过
覆盖率预检。Node 与浏览器的 `glyph()` plugin 和 presets 也接受相同策略。
仅按 GID 请求时不需要 Unicode selector；越界 ID 也遵循同一个缺字策略。

当 `trim: false` 时，校验后的源 TTF 字节会原样返回，因此不会应用其他子集策略。
启用裁剪后，`keepNotdef: false` 仍保留格式要求的 glyph zero 槽位和度量，但会把其
轮廓替换为空轮廓。

`keepLayout: 'drop'` 会移除 `GDEF`、`GPOS` 和 `GSUB`。默认的
`conservative` 会把受支持的 layout 数据重映射到新的 glyph ID，并可能丢弃已经无法
匹配的 contextual subtables。`preserve` 执行相同重映射，但会拒绝已知的
contextual-subtable 丢失和不受支持的 FeatureVariations，并提示改用
`conservative` 或 `drop`。
`layoutFeatures`、`layoutScripts` 与 `layoutLanguages` 可继续限制可达的
GSUB/GPOS layout 链。使用 `default` 保留 DefaultLangSys；`ENG` 这类三字符语言
tag 会自动补成四字节。
`nameIds` 与 `nameLanguages` 会分别筛选 `name` 表记录，并按 AND 语义组合。language
ID 与 platform 相关；例如 Windows 英语在 JavaScript 中可写成 `0x0409`，在 JSON
中写成 `1033`。

`dropTables` 不能移除必需的轮廓、度量、映射或命名表；`CBDT`/`CBLC` 与
`vhea`/`vmtx` 必须成对移除。`passThroughTables` 不允许覆盖子集引擎已经重写的表，也
拒绝在改写后必然失效的 `DSIG`。AAT、Graphite、bitmap index 与 `BASE` 等已知含 glyph
index 的格式默认丢弃，显式透传时要求 `retainGids: true`；调用方仍需确认没有引用已
移除的 glyph。明确指定未知 tag 表示调用方确认该自定义表仍然有效。源字体不存在的 tag
不会报错；在 `keepLayout: 'preserve'` 下移除 layout 表会因语义冲突而报错。

## 输出选项

Rust 配置文件使用输出对象。Node programmatic config 还接受 `'woff2'` 这样的格式
字符串作为简写。

| 字段       | 说明                                          |
| ---------- | --------------------------------------------- |
| `format`   | `ttf`、`woff`、`woff2`、`eot`、`svg` 或 `css` |
| `clone`    | 在转换产物之外保留输入资产；默认值为 true     |
| `fileName` | 覆盖生成的文件名                              |
| `ext`      | 覆盖生成的扩展名                              |

当前由请求的输出格式和每次转换的 `clone` 选项控制产物保留；CLI 的
`--no-original` 会移除请求中的 TTF 输出。`preserveOriginal` 为兼容性保留在两套
配置结构中，但目前不会作为独立的输出过滤器应用。

## Unicode 分片交付

设置 `delivery.slices` 后，会在所选格式转换前为每个具名范围组生成一份子集。分片名必须唯一，且只能包含字母、数字、连字符或下划线。每个 `unicodeRanges` 条目都接受 `U+HEX` 或 `U+HEX-HEX` 形式；每个端点使用一到六位十六进制数字。

上面的示例会生成 `roboto-regular-latin.*` 和
`roboto-regular-cjk.*`。CSS 输出会为每个分片写入独立的 `unicode-range` 描述符，并优先于该来源的全局 CSS `unicodeRanges` 选项。

`delivery` 是 Rust 配置字段。Node pipeline 应在格式转换和 CSS plugin 之前放置
`deliverySlices([...])`。

## 可变字体与 CFF/CFF2 输入

Rust 构建引擎会在子集化与 Web 转换前，将受支持的 OTF 输入规范化为静态
TrueType。通过 `otf.variationCoordinates` 可完整实例化 glyf variable TTF 或
CFF2 OTF。重复的 `build --variation TAG=VALUE` 会覆盖该对象中同名轴的值，同时
保留其他已配置轴；未指定的轴使用默认值。静态输出不会保留 variation tables、
TrueType hinting 或 Type 2 hinting。

为兼容 Fontmin，`preserveHinting` 仍可传入：glyf-backed OTF wrapper 会保留源
instructions；CFF/CFF2 转换无法把 Type 2 hinting 翻译为 TrueType，因此两个取值
产生相同输出。同样，`svg2ttf({ hinting: true })` 仍被接受，但不会生成 TrueType
hint instructions。

Node 配置没有顶层 `otf` 字段；请将相同的 `variationCoordinates` 传给
`modernWeb()` 或 `otf2ttf()`；后者在存在坐标时也会实例化 variable TTF asset。

局部缩减可使用 `modernWeb({ variationAxes })`，或组合 `variationSpace()` 插件。
JSON/Rust module 配置可以使用对应的 native descriptor：

```json
{
  "name": "fontmin:variation-space",
  "native": {
    "kind": "builtin",
    "name": "variationSpace",
    "options": {
      "axes": {
        "wdth": 100,
        "wght": { "min": 300, "max": 700, "default": 500 }
      },
      "clone": false
    }
  }
}
```

数值会固定轴，范围对象会保留缩窄后的轴，未列出的轴继续保持可变。包装字体会规范化
为 TTF/OTF SFNT 输出。

## CSS 选项

| 字段            | 说明                                                     |
| --------------- | -------------------------------------------------------- |
| `fontFamily`    | `@font-face` 的 `font-family`；Node 也接受 resolver 函数 |
| `fontPath`      | CSS 中字体文件路径前缀                                   |
| `fontDisplay`   | `font-display` 值                                        |
| `local`         | 是否生成 local source                                    |
| `glyph`         | 生成 icon glyph class 规则                               |
| `iconPrefix`    | 生成 glyph class 时使用的 class 前缀                     |
| `asFileName`    | 使用 SVG icon 文件名作为 class 后缀                      |
| `base64`        | 是否内联字体内容                                         |
| `target`        | CSS、SCSS 或 Less 输出目标                               |
| `unicodeRanges` | 当来源未定义范围时使用的全局 `unicode-range` 描述符      |
