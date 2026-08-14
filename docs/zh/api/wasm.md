# 浏览器 WASM API

`@fontmin-rs/wasm` 可以在浏览器内完整运行 fontmin-rs。它是异步、纯内存 API：输入和输出均为 `Uint8Array`，不依赖 Node.js native binding。

## 安装与初始化

```sh
pnpm add @fontmin-rs/wasm
```

使用任意转换或流水线 API 前，先初始化一次。普通 bundler 或浏览器 ESM 导入会自动加载同目录的 `.wasm` 文件；如果你的构建方式需要自行处理静态资源，也可以显式传入 WASM 字节或 URL。

```ts
import { initWasm, isWasmInitialized } from '@fontmin-rs/wasm'

await initWasm()
console.log(isWasmInitialized()) // true
```

重复调用 `initWasm()` 会复用同一个初始化 Promise。`isWasmInitialized()` 仅用于同步
状态检查；开始处理字体前仍应等待 `initWasm()` 完成。

## 直接转换

所有直接 API 都返回 `Promise`，并处理内存数据：

| API                                                | 能力                                       |
| -------------------------------------------------- | ------------------------------------------ |
| `analyzeCoverage(input, options)`                  | 报告请求、支持与缺失的 Unicode 码点。      |
| `subsetTtf(input, options)`                        | 根据文本、Unicode 或原始 GID 子集化 TTF。  |
| `subsetTtfWithReport(input, options)`              | 子集化 TTF，并返回体积、表与字形映射详情。 |
| `ttfToWoff(input, options)` / `woffToTtf(input)`   | TTF 与 WOFF 1.0 互转。                     |
| `ttfToWoff2(input, options)` / `woff2ToTtf(input)` | TTF 与 WOFF2 互转。                        |
| `validateWoff2(input)`                             | 校验 WOFF2 header 与 table directory。     |
| `ttfToEot(input, options)` / `eotToTtf(input)`     | TTF 与 EOT 互转。                          |
| `ttfToSvg(input, options)`                         | TTF 转 SVG 字体字符串。                    |
| `svgFontToTtf(input, options)`                     | SVG 字体字符串转 TTF。                     |
| `svgsToTtf(icons, options)`                        | 多个 SVG 图标生成 TTF 图标字体。           |
| `instantiateFont(input, options)`                  | 固定可变字体全部轴并输出静态 TTF。         |
| `otfToTtf(input, options)`                         | 静态 CFF OTF 或 CFF2 OTF 实例转 TTF。      |
| `inspect(input)`                                   | 读取格式与字体元信息。                     |
| `inspectCapabilities(input)`                       | 报告结构化彩色字体子集支持状态。           |
| `inspectCollection(input)`                         | 列出 TTC/OTC 集合中的全部 face。           |
| `extractCollectionFace(input, faceIndex)`          | 按从 0 开始的索引提取独立 SFNT face。      |
| `generateFontFaceCss(sources, options)`            | 生成 `@font-face` CSS。                    |

`inspectCollection()` 与 `extractCollectionFace()` 异步提供和 Node API
一致的 TTC/OTC 元信息与独立 TTF/OTF 提取，全程只使用浏览器内存。提取结果可
直接传给其他 WASM 转换 API。

能力报告与 Node 使用相同的 `subset`、`passthrough`、`unsupported` 状态，
并明确区分 COLR v0/v1，也会诊断缺失配对表的输入。

SVG Font 与 icon 转换也和 Node 保持一致：浏览器内存 API 支持平滑曲线
`S`/`T`、椭圆弧 `A`、相对命令，以及通过 cmap format 12 编码的补充平面
Unicode 码点。

```ts
import {
  analyzeCoverage,
  initWasm,
  subsetTtf,
  subsetTtfWithReport,
  ttfToWoff2,
  validateWoff2,
} from '@fontmin-rs/wasm'

await initWasm()

const ttf = new Uint8Array(
  await (await fetch('/fonts/roboto.ttf')).arrayBuffer(),
)
const coverage = await analyzeCoverage(ttf, { text: 'A𠮷' })
const subset = await subsetTtf(ttf, { text: 'Hello' })
const woff2 = await ttfToWoff2(subset)

await validateWoff2(woff2)
console.log(coverage.missing)
```

`analyzeCoverage()` 返回 `coveragePercent` 与排序后的 `requested`、
`supported`、`missing` 数组。`subsetTtf()` 和 glyph presets 接受
`missingGlyphs: 'ignore' | 'warn' | 'error'`；默认的 `warn` 会调用
`console.warn`，`error` 会在子集化前拒绝不完整覆盖。

传入 `gids: [1, 7]` 可以直接保留原始 glyph ID，无需 Unicode selector。
传入 `glyphNames: ['A', 'space']` 可选择精确的 PostScript glyph 名；字体未保存名称时
可使用稳定生成的 `gidDDD` 名称。

`await subsetTtfWithReport(input, options)` 接受相同 selector，并返回
`{ data, report }`。报告包含源文件与子集体积、保留的表和字形数、请求/支持/缺失
的 GID 与 glyph 名、glyph 名到原始 GID、旧新 GID 双向映射，以及 Unicode 到原始
GID 的映射。

`preserveHinting`、`keepNotdef`、`retainGids`、`layout` 和 `trim` 与 native helpers
具有相同的可观察语义。保留 ID 时，空的中间槽位在 `report.newToOld` 中表示为
`null`。`retainGlyphNames: true` 会按新 GID 顺序输出 version 2 `post` 表；默认的
version 3 表不含名称。`retainLegacyCmap` 与 `retainSymbolCmap` 会重映射显式保留的
源 encoding 记录，支持 format 0、4、6、10、12 和 13，输出归一化为 format 4 或
12。发现 contextual layout 数据会丢失或 FeatureVariations 不受支持时，
`layout: 'preserve'` 会报错，而不会静默降级。

`layoutFeatures`、`layoutScripts` 和 `layoutLanguages` 会同时筛选 GSUB 与 GPOS 的
OpenType tag。`default` 表示 DefaultLangSys，三字符语言 tag 会自动补空格；空数组
表示保留全部 tag。

`nameIds` 与 `nameLanguages` 用于筛选 OpenType `name` 记录；后者使用与 platform
相关的数值 language ID。空数组表示全部保留，同时设置时按 AND 语义组合。

`dropTables` 会在重写后移除指定的可选表，`passThroughTables` 则原样恢复明确指定的
源表；两者都使用四字节可打印 ASCII tag。必需表、已重写表与 `DSIG` 会被拒绝，已知
含 glyph index 的透传表要求启用 `retainGids`。

`instantiateFont(input, { variationCoordinates })` 接受 glyf-backed TTF、
WOFF、WOFF2、EOT 或 CFF2 OTF。它会固定全部轴，未指定的 tag 使用 `fvar` 默认值，
并返回 glyph ID 稳定的静态 TTF。未知、非有限值与越界坐标会报错。完成求值后会移除
variation 与 TrueType hinting tables。

`reduceVariationSpace(input, { axes, downgradeCff2 })` 则会让未列出的轴继续保持可变。
`axes` 中的值可以是固定轴的数值，也可以是 `{ min, max, default? }`；省略范围默认值
时，会把原默认值夹到新范围内。`variationSpace()` 提供对应的内存浏览器插件，
`modernWeb({ variationAxes })` 会在子集化与 Web 输出前插入该处理。

OTF 的 `preserveHinting` 与 SVG 的 `hinting` 字段仍作为兼容选项接受。CFF/CFF2
Type 2 hints 不会被翻译，SVG 转换也不会生成 TrueType hint instructions。

`generateFontFaceCss()` 接收内存中的具名字体来源。设置 `base64: true` 可将字体字节嵌入为 data URL。

## 诊断

当 Rust core 返回结构化失败时，直接 helpers 和内置插件会以
`FontminDiagnosticError` 拒绝 Promise。其中 `code` 是稳定、可供程序读取的值，
例如 `fontmin::invalid_font`；对于共享的 malformed input corpus，`message` 与
Node native runtime 保持一致。

```ts
import { FontminDiagnosticError, initWasm, inspect } from '@fontmin-rs/wasm'

await initWasm()

try {
  await inspect(new Uint8Array([0]))
} catch (error) {
  if (error instanceof FontminDiagnosticError) {
    console.error(error.code, error.message)
  }
}
```

malformed input 会作为错误被拒绝，不会以 Rust panic 穿过 WASM API。初始化、
browser plugin 和不来自 Rust 诊断层的 JavaScript 选项错误会保留原有错误类型。

## 内存流水线

`optimizeBrowser()` 将插件应用到具名内存资产，并返回转换和新生成的资产；下载、缓存或上传输出由应用自行处理。

```ts
import { initWasm, modernWeb, optimizeBrowser } from '@fontmin-rs/wasm'

await initWasm()

const assets = await optimizeBrowser({
  assets: [{ contents: ttf, fileName: 'roboto.ttf' }],
  plugins: modernWeb({
    text: 'Hello browser',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})

const woff2 = assets.find(asset => asset.fileName === 'roboto.woff2')
const css = assets.find(asset => asset.fileName === 'roboto.css')
```

内置插件包括 `glyph`、`deliverySlices`、`ttf2woff`、`ttf2woff2`、`ttf2eot`、
`ttf2svg`、`otf2ttf`、`svg2ttf`、`svgs2ttf` 和 `css`。

- `modernWeb()` 会先将受支持的 CFF/CFF2 OTF 输入规范化为静态 TTF，再组合子集化、WOFF、WOFF2 与 CSS 输出。传入 `variationCoordinates` 可选择 CFF2 实例；源 OTF 会被替换。
- `fontminCompatPreset()` 在此基础上增加 OTF 转换、EOT 与 SVG 输出，得到经典 Fontmin 兼容产物组。
- `css({ base64: true })` 会内嵌流水线中的字体字节。

### Unicode 分片交付

`deliverySlices()` 会把每个 TTF 资产替换为每个具名范围对应的一份子集，并为 CSS
生成保留这些范围：

```ts
import {
  css,
  deliverySlices,
  optimizeBrowser,
  ttf2woff2,
} from '@fontmin-rs/wasm'

const assets = await optimizeBrowser({
  assets: [{ contents: ttf, fileName: 'roboto.ttf' }],
  plugins: [
    deliverySlices([
      { name: 'latin', unicodeRanges: ['U+0000-00FF'] },
      { name: 'cjk', unicodeRanges: ['U+4E00-9FFF'] },
    ]),
    ttf2woff2(),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})
```

分片名必须唯一，且只能包含字母、数字、连字符或下划线；每个分片至少需要一个范围。

## 自定义插件

浏览器插件可以转换资产、生成额外资产及报告警告，但不能访问文件系统。

```ts
const rename = {
  name: 'example:rename',
  transform(asset, context) {
    context.warn(`processing ${asset.fileName}`)
    context.emitFile({
      contents: new Uint8Array([1]),
      fileName: 'manifest.bin',
    })
    return { ...asset, fileName: `web-${asset.fileName}` }
  },
}
```

浏览器插件接口刻意小于 Node 流水线：不支持 `buildStart`、`generateBundle` 和 `buildEnd` hooks。

## 运行时边界与浏览器支持

此包不支持路径输入、glob 展开、CLI、磁盘缓存、输出目录或 Node.js 文件系统 hooks。请在应用中获取输入，并在内存中处理返回字节。

浏览器验收测试会在 Chromium、Firefox 和 WebKit 中通过 `FontFace` 加载生成的 WOFF2 字节。
