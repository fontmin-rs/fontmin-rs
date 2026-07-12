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
  ttfToWoff2Async,
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

`ttfToWoff2(input, { fallback })` 保持同步且仅使用 native。它支持 `native` 和 `auto`；`fallback: 'wasm'` 会提示 WASM 路径是异步的。

当 native artifact 可能不可用时，使用 `ttfToWoff2Async()`。它只会在请求时加载随包发布的 WASM runtime。`fallback: 'wasm'` 始终使用 WASM；`fallback: 'auto'` 先尝试 native binding，并且只在 binding 无法加载时回退。无效字体数据和 native encoder 错误会直接返回，不会使用 WASM 重试。

```ts
const woff2 = await ttfToWoff2Async(input, { fallback: 'auto' })
```

`fallback: 'js'` 仍不受支持。低层 helper 的这些 fallback 选项与下文基于文件的 `optimize()` pipeline runtime 选择相互独立。

`validateWoff2(input)` 会校验 WOFF2 header 和 table directory；有效输入正常返回，无效数据会抛错。`inspect(woff2)` 会先执行同样的校验，再读取 `name`、`head`、`hhea`、`maxp` 等 sfnt metadata tables。`woff2ToTtf(input)` 会通过 native binding 将 WOFF2 解码回 TTF。

## Browser WASM API

浏览器端处理请使用独立的[浏览器 WASM API](./wasm)。其中包含初始化、直接转换、内存流水线、自定义浏览器插件，以及浏览器运行时边界说明。

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
