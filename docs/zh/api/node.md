# Node API

`fontmin-rs` 的 Node API 包含四部分：

- 低层 native helpers，直接处理 `Uint8Array`。
- 用于类型化项目配置的 `defineConfig()` 与 `loadConfig()`。
- `optimize(config)` pipeline，处理输入文件、插件、缓存和输出。
- Fontmin-compatible 默认导出，适合迁移现有 Fontmin 链式调用。

## Native helpers

```ts
import {
  analyzeCoverage,
  eotToTtf,
  generateFontFaceCss,
  inspect,
  otfToTtf,
  subsetTtf,
  subsetTtfWithReport,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  ttfToWoff2Async,
  validateWoff2,
  woff2ToTtf,
  woffToTtf,
} from 'fontmin-rs'
import { readFileSync, writeFileSync } from 'node:fs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const coverage = analyzeCoverage(input, { text: 'A𠮷' })
const subset = subsetTtf(input, { text: 'Hello' })
const woff2 = ttfToWoff2(subset)
validateWoff2(woff2)
const decodedWoff2 = woff2ToTtf(woff2)
const info = inspect(woff2)

writeFileSync('build/roboto-subset.woff2', woff2)
writeFileSync('build/roboto-decoded-woff2.ttf', decodedWoff2)
console.log(info.format)
console.log(coverage.missing)
```

| Helper                                             | 能力                                        |
| -------------------------------------------------- | ------------------------------------------- |
| `analyzeCoverage(input, options)`                  | 报告请求、支持与缺失的 Unicode 码点。       |
| `subsetTtf(input, options)`                        | 按文本、Unicode 选择或原始 GID 子集化 TTF。 |
| `subsetTtfWithReport(input, options)`              | 子集化 TTF，并返回体积、表与字形映射详情。  |
| `ttfToWoff(input, options)` / `woffToTtf(input)`   | TTF 与 WOFF 1.0 互转。                      |
| `ttfToWoff2(input, options)` / `woff2ToTtf(input)` | TTF 与 WOFF2 互转。                         |
| `ttfToWoff2Async(input, options)`                  | 使用可选 native/WASM fallback 编码 WOFF2。  |
| `validateWoff2(input)`                             | 校验 WOFF2 header 与 table directory。      |
| `ttfToEot(input, options)` / `eotToTtf(input)`     | TTF 与 EOT 互转。                           |
| `ttfToSvg(input, options)`                         | 将 TTF 转为 SVG font 字符串。               |
| `svgFontToTtf(input, options)`                     | 将 SVG font 字符串转为 TTF。                |
| `svgsToTtf(icons, options)`                        | 将多个 SVG 图标生成 TTF 图标字体。          |
| `instantiateFont(input, options)`                  | 固定可变字体全部轴并输出静态 TTF。          |
| `otfToTtf(input, options)`                         | 将静态 CFF OTF 或 CFF2 OTF 实例转换为 TTF。 |
| `inspect(input)`                                   | 检测格式并读取字体元信息。                  |
| `generateFontFaceCss(sources, options)`            | 从具名字体来源生成 `@font-face` CSS。       |

`analyzeCoverage()` 接受与子集化相同的 `text`、`unicodes`、
`unicodeRanges` 与 `basicText` selector，并返回 `coveragePercent` 以及排序后的
`requested`、`supported`、`missing` 数组。`subsetTtf()` 与 glyph presets 接受
`missingGlyphs: 'ignore' | 'warn' | 'error'`；默认的 `warn` 会发出代码为
`FONTMIN_MISSING_GLYPHS` 的 process warning，`error` 会在子集化前拒绝不完整覆盖。

`subsetTtf(input, { gids: [1, 7] })` 可直接选择原始 glyph ID，也可以与文本、
码点或范围组合。`glyphNames: ['A', 'space']` 可选择精确的 PostScript glyph 名；
字体未保存名称时可使用稳定生成的 `gidDDD` 名称。

`subsetTtfWithReport()` 接受相同选项并返回 `{ data, report }`。报告包含源文件与
子集体积、保留的表和字形数、请求/支持/缺失的 GID 与 glyph 名、glyph 名到原始
GID、旧新 GID 双向映射，以及子集使用的 Unicode 到原始 GID 映射，可用于生成交付
清单、字形诊断和缓存元数据。

子集策略在 native 与 WASM 中都具有相同的可观察语义：`preserveHinting` 保留
`cvt `、`fpgm` 和 `prep`，`keepNotdef: false` 输出空的 glyph-zero 轮廓，
`retainGids: true` 保留原始 ID 并输出空的中间 glyph 槽位；这些空槽在
`report.newToOld` 中表示为 `null`。`retainGlyphNames: true` 会按新 GID 顺序重写
version 2 `post` 表；默认的 version 3 表会省略名称以缩小输出。`retainLegacyCmap`
与 `retainSymbolCmap` 可显式保留默认 Unicode-only `cmap` 会省略的源 encoding 记录，
并把仍存活的映射重写到新 GID；支持源 format 0、4、6、10、12 和 13，输出会归一化
为 format 4 或 12，同时保留 record identity 与 language。`keepLayout` 用于选择丢弃、保守重映射或严格
layout 处理。严格模式会拒绝已知的 contextual 数据丢失和不受支持的
FeatureVariations，不会静默降级。`trim: false` 会原样返回校验后的源字节。

`layoutFeatures`、`layoutScripts` 和 `layoutLanguages` 会对白名单中的 OpenType tag
同时应用 GSUB 与 GPOS 裁剪。空数组或省略字段表示全部保留；在 `layoutLanguages` 中
使用 `default` 选择各 script 的 DefaultLangSys，`ENG` 这类三字符语言 tag 会自动补空格。

`nameIds` 与 `nameLanguages` 用于筛选 OpenType `name` 记录。语言 ID 是与 platform
相关的数值（例如 Windows 英语为 `0x0409`）。空数组或省略字段表示全部保留；两个
筛选器同时存在时，记录必须同时匹配。筛选后 format 1 的 language-tag 索引仍保持有效。

`dropTables` 会在常规重写后移除指定的可选表；`passThroughTables` 会把明确指定的源表
原样复制，并重新计算 SFNT checksum。tag 必须是四个可打印 ASCII 字节。必需表和已经
由子集引擎重写的表不能覆盖，`DSIG` 不能保留，已知含 glyph index 的透传表要求
`retainGids: true`。源字体不存在的 tag 会忽略，明确指定的未知 tag 视为调用方确认过的
自定义 metadata。

`ttfToWoff(input, options)` 支持通过 `metadata` XML 和 `privateData` 字节写入 WOFF 1.0 附加 block。metadata 会在 WOFF 文件中使用 zlib 压缩，private data 会作为最后一个 block 原样存储。

`ttfToWoff2(input, { fallback })` 保持同步且仅使用 native。它支持 `native` 和 `auto`；`fallback: 'wasm'` 会提示 WASM 路径是异步的。

当 native artifact 可能不可用时，使用 `ttfToWoff2Async()`。它只会在请求时加载随包发布的 WASM runtime。`fallback: 'wasm'` 始终使用 WASM；`fallback: 'auto'` 先尝试 native binding，并且只在 binding 无法加载时回退。无效字体数据和 native encoder 错误会直接返回，不会使用 WASM 重试。

```ts
const woff2 = await ttfToWoff2Async(input, { fallback: 'auto' })
```

`fallback: 'js'` 仍不受支持。低层 helper 的这些 fallback 选项与下文基于文件的 `optimize()` pipeline runtime 选择相互独立。

`validateWoff2(input)` 会校验 WOFF2 header 和 table directory；有效输入正常返回，无效数据会抛错。`inspect(woff2)` 会先执行同样的校验，再读取 `name`、`head`、`hhea`、`maxp` 等 sfnt metadata tables。`woff2ToTtf(input)` 会通过 native binding 将 WOFF2 解码回 TTF。

## 诊断

native helpers 和使用 native 的内置插件在 fontmin-rs 返回结构化失败时会抛出
`FontminDiagnosticError`。其中 `code` 是稳定、可供程序读取的值，例如
`fontmin::invalid_font`；`message` 则包含面向用户的错误详情。对于共享的 malformed
input corpus，native 与强制 WASM runtime 会返回相同的 code 和 message。

```ts
import { FontminDiagnosticError, inspect } from 'fontmin-rs'

try {
  inspect(new Uint8Array([0]))
} catch (error) {
  if (error instanceof FontminDiagnosticError) {
    console.error(error.code, error.message)
  }
}
```

malformed input 会作为错误被拒绝，不会以 Rust panic 穿过公共 API。runtime
加载、JavaScript 插件和不来自 Rust 诊断层的选项校验失败会保留原有错误类型。

## Browser WASM API

浏览器端处理请使用独立的[浏览器 WASM API](./wasm)。其中包含初始化、直接转换、内存流水线、自定义浏览器插件，以及浏览器运行时边界说明。

## 配置 helpers

使用 `defineConfig()` 可获得对象配置的类型检查；`loadConfig()` 可以加载显式路径，或
自动发现第一个受支持的 `fontmin.config.*` 文件。未设置 `cwd` 时，`loadConfig()` 会
将其设为配置文件目录，使相对输入、输出路径、缓存路径和 `textFile` 都以项目配置为基准。

```ts
import { defineConfig, loadConfig, modernWeb, optimize } from 'fontmin-rs'

const config = defineConfig({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  plugins: modernWeb({ text: 'Hello' }),
})

await optimize(config)
```

如果要自动发现并运行配置文件，可在项目脚本中调用
`await optimize(await loadConfig())`。

配置发现、可执行 module 的安全边界，以及 Rust CLI 与 Node 配置模型的差异，请参阅
[配置文件](../guide/config)。

## optimize

```ts
import { modernWeb, optimize } from 'fontmin-rs'

await optimize({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  runtime: 'auto',
  plugins: modernWeb({ text: 'Hello' }),
})
```

### Pipeline runtime

`runtime` 控制一次 `optimize()` 调用中的全部内置字体操作：

- `native` 是默认值，并要求对应平台的 native binding 可用。
- `wasm` 加载随包发布的 WASM module，并强制所有内置操作使用它。
- `auto` 在 binding 可加载时选择 native，否则选择 WASM。它只会因 native binding 加载错误而回退；无效输入、不支持的选项和转换错误会直接返回，不会使用 WASM 重试。

整个 pipeline 只选择一个 runtime；内置操作不会混用 native 和 WASM。输入发现、文件读写、缓存和自定义 JavaScript plugin hook 仍在 Node 中运行，只有内置字体操作会跨越所选的 native 或 WASM 边界。

为了兼容旧配置，当没有设置 `runtime` 时，内置 `ttf2woff2()` plugin 的 `fallback` 可以选择 pipeline runtime。完整兼容矩阵如下：

| `runtime`                  | `ttf2woff2({ fallback })`                              | 结果                                |
| -------------------------- | ------------------------------------------------------ | ----------------------------------- |
| 省略                       | 省略                                                   | 选择 `native`                       |
| `native`、`wasm` 或 `auto` | 省略                                                   | 选择配置的 runtime                  |
| 省略                       | `native`、`wasm` 或 `auto`                             | 将 fallback 值作为 pipeline runtime |
| 某一模式                   | 相同模式                                               | 选择该模式                          |
| 某一模式                   | 不同模式                                               | 抛出 runtime/fallback 冲突错误      |
| 任意值                     | `js`                                                   | 抛出不支持 fallback 的错误          |
| 任意值                     | 多个 plugin 使用不止一种 `native`、`wasm` 或 `auto` 值 | 抛出 fallback 模式冲突错误          |

## modernWeb preset

```ts
import { modernWeb, optimize } from 'fontmin-rs'

await optimize({
  input: ['fonts/*.ttf'],
  outDir: 'build',
  runtime: 'auto',
  plugins: modernWeb({ text: 'Hello' }),
})
```

`modernWeb()` 会先将受支持的 CFF/CFF2 OTF 输入规范化为静态 TTF，再组合
`glyph()`、`ttf2woff()`、`ttf2woff2()` 和 `css()`。传入
`variationCoordinates` 会在子集化前完整实例化 glyf variable TTF 或 CFF2 OTF；
未指定的轴使用默认值。它不会生成 EOT 或 SVG；如需这些格式，请显式添加
`ttf2eot()` 或 `ttf2svg()`。

如果输出仍需保持可变，请改用 `variationAxes`。数值用于固定轴，范围对象用于缩窄
保留轴，未列出的轴继续保持可变：

```ts
modernWeb({
  variationAxes: {
    wdth: 100,
    wght: { min: 300, max: 700, default: 500 },
  },
})
```

## Fontmin 兼容 preset

```ts
import { fontminCompatPreset, optimize } from 'fontmin-rs'

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  plugins: fontminCompatPreset({
    text: 'Hello',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})
```

`fontminCompatPreset()` 使用经典 Fontmin 顺序：`otf2ttf()`、`glyph()`、`ttf2eot()`、`ttf2svg()`、`ttf2woff()`、`ttf2woff2()` 和 `css()`。它也可以从 `fontmin-rs/presets` 子路径导入。

`otfToTtf()` 可以将静态 CFF OpenType 字体，或 CFF2 可变字体的默认/显式实例转换为静态 TrueType `glyf` 字体。可以使用 `variationCoordinates` 传入用户空间坐标：

```ts
otfToTtf(input, { variationCoordinates: { wght: 700, opsz: 14 } })
```

输出保留 glyph ID、cmap 映射、度量、名称和支持的 OpenType layout 表；CFF2 和 variation 表会被移除，Type 2 hinting 会被丢弃。

输入本身是可变字体时使用 `instantiateFont()`。它接受 glyf-backed TTF、WOFF、
WOFF2、EOT 或 CFF2 OTF，并始终返回一份静态 TTF：

```ts
const staticBold = instantiateFont(variableFont, {
  variationCoordinates: { wght: 700 },
})
```

所有轴都会被固定；未指定的轴使用 `fvar` 默认值。未知、非有限值或越界坐标会报错，
不会被静默截断。glyph ID 保持稳定；variation tables 与 TrueType hinting programs
会在完成求值后移除，因为它们不再描述静态轮廓。

`reduceVariationSpace()` 可在保留可变字体的同时固定部分轴或缩窄轴范围。它接受
TTF、OTF、WOFF、WOFF2 与 EOT；包装格式输入会返回未包装的 SFNT。所有轴都固定
时，可设置 `downgradeCff2: true` 将 CFF2 转为 CFF1。

```ts
const reduced = reduceVariationSpace(variableFont, {
  axes: {
    wdth: 100,
    wght: { min: 300, max: 700 },
  },
})
```

`otfToTtf({ preserveHinting: true })` 与
`svgFontToTtf({ hinting: true })` 仍作为兼容选项接受。前者无法翻译 CFF/CFF2 Type 2
hints，后者不会生成 TrueType instructions，因此这些取值不会改变转换后的轮廓。

## 插件

内置工厂包括 `glyph`、`deliverySlices`、`autoDeliverySlices`、`variationSpace`、
`otf2ttf`、`ttf2woff`、`ttf2woff2`、`ttf2eot`、`ttf2svg`、`svg2ttf`、
`svgs2ttf`、`css` 和 `webDelivery`。它们可以从包根入口或
`fontmin-rs/plugins` 子路径导入。

`variationSpace(options)` 将同一能力作为可组合流水线插件提供。默认替换输入；设置
`clone: true` 会生成 `*-reduced.ttf` 或 `*-reduced.otf` 副本。

为保持兼容，`otf2ttf()` 沿用原名称；但设置 `variationCoordinates` 后，它也会
实例化 variable TTF asset。默认 `clone: true` 时静态副本命名为
`*-instance.ttf`；使用 `clone: false` 可原位替换可变输入。

### Unicode 分片交付

`deliverySlices()` 会把每个 TTF 资产替换为每个具名 Unicode 范围对应的一份子集。
请将它放在所需的 OTF 标准化之后、格式转换与 CSS 生成之前。每个分片的范围会进入
生成的 `unicode-range` 描述符。

```ts
import { css, deliverySlices, optimize, ttf2woff2 } from 'fontmin-rs'

await optimize({
  input: ['fonts/roboto.ttf'],
  outDir: 'build',
  plugins: [
    deliverySlices([
      { name: 'latin', unicodeRanges: ['U+0000-00FF'] },
      { name: 'cjk', unicodeRanges: ['U+4E00-9FFF'] },
    ]),
    ttf2woff2({ clone: false }),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})
```

分片名必须唯一，且只能包含字母、数字、连字符和下划线；每个分片至少需要一个
Unicode 范围。

### 自动分片与 Web 交付

`autoDeliverySlices()` 会根据语言/script 覆盖、高频文本、实测编码字节目标、容差与
最大请求数规划分片。顶层 `autoDelivery` 会在配置的格式输出之前插入同一 plugin。
多个 TTF face 共享一个计划；每个候选分组都会在所有匹配 face 上编码，并按其中最大
结果执行约束。

```ts
import { optimize } from 'fontmin-rs'

await optimize({
  input: ['fonts/noto-sans-sc-regular.ttf', 'fonts/noto-sans-sc-bold.ttf'],
  outDir: 'build',
  autoDelivery: {
    frequencyText: 'AB中文',
    languages: ['en', 'zh-Hans'],
    targetBytes: 100 * 1024,
    tolerance: 0.15,
    maxSlices: 16,
    measureFormat: 'woff2',
  },
  outputs: ['woff2'],
  webDelivery: {
    fontFamily: 'Noto Sans SC',
    basePath: '/assets/fonts',
    hashFileNames: true,
    hashLength: 12,
    testHtmlFile: 'fontmin-preview.html',
  },
})
```

`webDelivery` 会生成交付 CSS、preload 标记、原始完整字体 fallback 与 JSON manifest。
每个 manifest 资产都包含 SHA-256、字节数、格式、preload 策略和精确 Unicode 范围；
`summary` 还会汇总源字体、子集和 fallback 体积，以及请求、产物和码点数量。
`hashFileNames` 会同时改写交付 CSS 和此前 pipeline CSS 中的引用；`testHtmlFile`
会增加可独立打开的表格/预览页。两项均为 opt-in，因此默认输出名称与文件集合不变。

### 自定义插件

```ts
import { definePlugin, optimize } from 'fontmin-rs'

const report = definePlugin({
  name: 'example:report',
  generateBundle(assets) {
    for (const asset of assets) {
      console.log(asset.path, asset.format, asset.contents.byteLength)
    }
  },
})

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  plugins: [report],
})
```

插件可以实现 `buildStart`、`transform`、`generateBundle` 和 `buildEnd`。内置插件通过 pipeline 所选的 runtime 执行核心字体操作；自定义插件仍在 Node 中运行，适合做重命名、报告、额外文件生成和项目内集成。

每个 hook 都会收到 `PluginContext`，包含 `cwd`、`resolve(path)`、`readFile(path)`、`writeFile(path, contents)`、`emitFile(asset)`、`warn(message)` 和 `diagnostics`。相对路径会基于 `cwd` 解析，`writeFile` 会自动创建父目录。

```ts
const manifest = definePlugin({
  name: 'example:manifest',
  async generateBundle(assets, ctx) {
    ctx.warn(`writing manifest for ${assets.length} assets`)
    await ctx.writeFile(
      'build/fontmin-manifest.json',
      JSON.stringify(
        assets.map(asset => ({
          format: asset.format,
          path: asset.path,
          size: asset.contents.byteLength,
        })),
        undefined,
        2,
      ),
    )
  },
})
```

## Fontmin-compatible chain

```ts
import Fontmin from 'fontmin-rs'

await new Fontmin()
  .src('fixtures/fonts/ttf/roboto-regular.ttf')
  .use(Fontmin.glyph({ text: 'Hello' }))
  .use(Fontmin.ttf2woff2())
  .dest('build')
  .runAsync()
```

这个入口面向迁移场景。新项目更推荐使用 `optimize(config)`，因为配置对象更容易序列化、缓存和测试。
