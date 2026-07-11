# Node API

`fontmin-rs` 的 Node API 分为三层：

- 低层 native helpers，直接处理 `Uint8Array`。
- `optimize(config)` pipeline，处理输入文件、插件、缓存和输出。
- Fontmin-compatible 默认导出，适合迁移现有 Fontmin 链式调用。

## Native helpers

```ts
import {
  eotToTtf,
  generateFontFaceCss,
  inspect,
  subsetTtf,
  svgFontToTtf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  validateWoff2,
  woff2ToTtf,
  woffToTtf,
} from 'fontmin-rs'
import { readFileSync, writeFileSync } from 'node:fs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const subset = subsetTtf(input, { text: 'Hello' })
const woff2 = ttfToWoff2(subset)
validateWoff2(woff2)
const decodedWoff2 = woff2ToTtf(woff2)
const info = inspect(woff2)

writeFileSync('build/roboto-subset.woff2', woff2)
writeFileSync('build/roboto-decoded-woff2.ttf', decodedWoff2)
console.log(info.format)
```

`ttfToWoff(input, options)` 支持通过 `metadata` XML 和 `privateData` 字节写入 WOFF 1.0 附加 block。metadata 会在 WOFF 文件中使用 zlib 压缩，private data 会作为最后一个 block 原样存储。

`ttfToWoff2(input, { fallback })` 支持 `native` 和 `auto`，目前两者都会使用 native binding。同步 Node API 不会自动加载 WASM；无法使用 native 模块时，请使用下面的浏览器专用包。

`validateWoff2(input)` 会校验 WOFF2 header 和 table directory；有效输入正常返回，无效数据会抛错。`inspect(woff2)` 会先执行同样的校验，再读取 `name`、`head`、`hhea`、`maxp` 等 sfnt metadata tables。`woff2ToTtf(input)` 会通过 native binding 将 WOFF2 解码回 TTF。

## Browser WASM API

浏览器端处理请使用独立的[浏览器 WASM API](./wasm)。其中包含初始化、直接转换、内存流水线、自定义浏览器插件，以及浏览器运行时边界说明。

## optimize

```ts
import { css, glyph, optimize, ttf2woff, ttf2woff2 } from 'fontmin-rs'

const assets = await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  cache: { enabled: true },
  plugins: [
    glyph({ text: 'Hello' }),
    ttf2woff(),
    ttf2woff2(),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})

console.log(assets.map(asset => asset.path))
```

## modernWeb preset

```ts
import { modernWeb, optimize } from 'fontmin-rs'

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  plugins: modernWeb({
    text: 'Hello',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})
```

`modernWeb()` 会先将受支持的 CFF/CFF2 OTF 输入规范化为静态 TTF，再组合 `glyph()`、`ttf2woff()`、`ttf2woff2()` 和 `css()`。传入 `variationCoordinates` 可选择 CFF2 实例，且不会输出源 OTF。它不会生成 EOT 或 SVG；如需这些格式，请显式添加 `ttf2eot()` 或 `ttf2svg()`。

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

## 插件

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

插件可以实现 `buildStart`、`transform`、`generateBundle` 和 `buildEnd`。内置插件通过 native binding 执行核心字体操作，自定义插件适合做重命名、报告、额外文件生成和项目内集成。

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
