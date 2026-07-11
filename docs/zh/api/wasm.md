# 浏览器 WASM API

`@fontmin-rs/wasm` 可以在浏览器内完整运行 fontmin-rs。它是异步、纯内存 API：输入和输出均为 `Uint8Array`，不依赖 Node.js native binding。

## 安装与初始化

```sh
pnpm add @fontmin-rs/wasm
```

使用任意转换或流水线 API 前，先初始化一次。普通 bundler 或浏览器 ESM 导入会自动加载同目录的 `.wasm` 文件；如果你的构建方式需要自行处理静态资源，也可以显式传入 WASM 字节或 URL。

```ts
import { initWasm } from '@fontmin-rs/wasm'

await initWasm()
```

## 直接转换

所有直接 API 都返回 `Promise`，并处理内存数据：

| API                                                | 能力                                   |
| -------------------------------------------------- | -------------------------------------- |
| `subsetTtf(input, options)`                        | 根据文本或 Unicode 对 TTF 做子集化。   |
| `ttfToWoff(input, options)` / `woffToTtf(input)`   | TTF 与 WOFF 1.0 互转。                 |
| `ttfToWoff2(input, options)` / `woff2ToTtf(input)` | TTF 与 WOFF2 互转。                    |
| `validateWoff2(input)`                             | 校验 WOFF2 header 与 table directory。 |
| `ttfToEot(input, options)` / `eotToTtf(input)`     | TTF 与 EOT 互转。                      |
| `ttfToSvg(input, options)`                         | TTF 转 SVG 字体字符串。                |
| `svgFontToTtf(input, options)`                     | SVG 字体字符串转 TTF。                 |
| `svgsToTtf(icons, options)`                        | 多个 SVG 图标生成 TTF 图标字体。       |
| `otfToTtf(input, options)`                         | 静态 CFF OTF 或 CFF2 OTF 实例转 TTF。  |
| `inspect(input)`                                   | 读取格式与字体元信息。                 |
| `generateFontFaceCss(sources, options)`            | 生成 `@font-face` CSS。                |

```ts
import {
  initWasm,
  subsetTtf,
  ttfToWoff2,
  validateWoff2,
} from '@fontmin-rs/wasm'

await initWasm()

const ttf = new Uint8Array(
  await (await fetch('/fonts/roboto.ttf')).arrayBuffer(),
)
const subset = await subsetTtf(ttf, { text: 'Hello' })
const woff2 = await ttfToWoff2(subset)

await validateWoff2(woff2)
```

`generateFontFaceCss()` 接收内存中的具名字体来源。设置 `base64: true` 可将字体字节嵌入为 data URL。

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

内置插件包括 `glyph`、`ttf2woff`、`ttf2woff2`、`ttf2eot`、`ttf2svg`、`otf2ttf`、`svg2ttf`、`svgs2ttf` 和 `css`。

- `modernWeb()` 会先将受支持的 CFF/CFF2 OTF 输入规范化为静态 TTF，再组合子集化、WOFF、WOFF2 与 CSS 输出。传入 `variationCoordinates` 可选择 CFF2 实例；源 OTF 会被替换。
- `fontminCompatPreset()` 在此基础上增加 OTF 转换、EOT 与 SVG 输出，得到经典 Fontmin 兼容产物组。
- `css({ base64: true })` 会内嵌流水线中的字体字节。

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
