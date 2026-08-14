# 从 Fontmin 迁移

`fontmin-rs` 保留 Fontmin 主流工作流，同时把耗时字体操作下沉到 Rust 和 N-API binding。这一页适合已经在构建脚本里使用 `fontmin`，并希望逐步迁移的项目。

## 从 0.3 升级到 1.0

`1.0` 不移除或重命名任何公开 API。经过独立验证的契约会保留 `0.3` 的 CLI command
与 flag、Node 与浏览器 exports、配置字段、稳定诊断码和生成文件命名规则。

安装稳定版本：

```sh
pnpm add fontmin-rs@latest
pnpm add @fontmin-rs/wasm@latest
```

如需复现稳定版提升证据，仍可安装经过审阅的候选版本：

```sh
pnpm add fontmin-rs@1.0.0-rc.1
pnpm add @fontmin-rs/wasm@1.0.0-rc.1
```

支持边界已记录在机器可读的
[`contracts/support.json`](https://github.com/fontmin-rs/fontmin-rs/blob/main/contracts/support.json)：

- Node.js 22.18、24、26 会阻断发布；package engine 仍为 `>=22.18.0`。
- `runtime: "native"` 仍是默认值；`"auto"` 只在 native binding 无法加载时回退
  WASM，不会在处理错误后重试。
- 继续支持相同的八个 native target、Chromium/Firefox/WebKit、Rust 1.88.0
  MSRV、诊断码和生成文件名模板。
- Fontmin-compatible 默认 export、`glyph({ hinting })` alias 和
  `ttf2woff2({ fallback })` runtime 兼容路径都会保留；它们均不符合 `1.0` 的移除
  条件。

升级生产构建前，请对选定的精确版本运行[独立兼容性项目](../compatibility.md)。

## 安装

```sh
pnpm add fontmin-rs
```

迁移期间可以继续保留 `fontmin`，按构建目标逐步替换。两个包名和 native 平台包相互独立，可以在同一个仓库中共存。

## 选择入口

如果想最小化代码改动，可以先使用 Fontmin-compatible chain：

```ts
import Fontmin from 'fontmin-rs'

await new Fontmin()
  .src('fonts/roboto.ttf')
  .use(Fontmin.glyph({ text: 'Hello' }))
  .use(Fontmin.ttf2woff2())
  .use(Fontmin.css({ fontFamily: 'Roboto', fontPath: './' }))
  .dest('build')
  .runAsync()
```

与经典 Fontmin 一样，兼容链在没有调用 `.use()` 时会输出原始 TTF，以及
EOT、WOFF、WOFF2、SVG font 和 CSS。只有显式加入 `Fontmin.glyph()` 时才会裁剪字形。

不传参数调用 `.src()` 或 `.dest()` 时，会返回最近一次在该兼容链上设置的参数，
与经典 Fontmin 的 getter 行为一致。

包也导出了经典的 `plugins`、`mime` 和 `util` 辅助对象。既可以使用 named export，
也可以通过兼容类上的 `Fontmin.plugins`、`Fontmin.mime` 和 `Fontmin.util` 访问。

兼容类上的 plugin factory 保留经典默认值：`Fontmin.glyph()` 保留 TrueType hinting，
`Fontmin.css()` 默认不添加 `local()` source，`Fontmin.otf2ttf()` 会替换 OTF 输入，
空的 `Fontmin.glyph()` 为 pass-through。named plugin export 继续使用现代 `fontmin-rs` 默认值。
在兼容类上，`Fontmin.css({ asFileName: true })` 会把源文件 stem 用作 `font-family`，
与经典 Fontmin 一致。

兼容版 `glyph` 插件也接受 Fontmin 的可变 `use(ttf)` 回调。如果它位于
`Fontmin.css()` 之前，`fontFamily(info, ttf)` 回调的第二个参数会收到改写后的 TTF 对象：

```ts
new Fontmin()
  .src('fonts/roboto.ttf')
  .use(Fontmin.glyph({
    text: 'Hello',
    use(ttf) {
      ttf.setName({ fontFamily: 'Roboto Subset' })
    },
  }))
  .use(Fontmin.css({
    fontFamily(info, ttf) {
      return ttf.name.fontFamily || info.fontFile
    },
  }))
```

这些可变回调是 Node 兼容功能，使用与锁定 Fontmin 基准相同的
`fonteditor-core@2.4.1` 对象模型。现代 named `glyph()` 和 `css()` 导出仍是类型化、
运行时无关的操作。

`run(callback)` 会返回 object-mode Node.js stream，同时保留 callback 结果。当前 data event
包含类型化 `FontAsset`；不需要 stream 时优先使用 `runAsync()`。

依赖 Vinyl 文件方法的旧 Gulp pipeline 和插件可以选择独立适配入口。它使用
`vinyl-fs` 处理 source/destination 选项、返回真正的 Vinyl 文件，并允许在类型化转换
插件之间插入普通 Vinyl Transform：

```ts
import { Transform } from 'node:stream'
import Fontmin from 'fontmin-rs/vinyl'

await new Fontmin()
  .src('fonts/*.ttf', { base: 'fonts' })
  .use(Fontmin.glyph({ text: 'Hello' }))
  .use(() => new Transform({ objectMode: true, transform(file, _, done) {
    file.stem = `${file.stem}-subset`
    done(null, file)
  }}))
  .dest('build', { overwrite: true })
  .runAsync()
```

Vinyl 适配器会缓冲每个类型化插件区段；不支持 `contents` 为 stream 的 Vinyl 文件，
请保留 `vinyl-fs.src()` 默认的 buffer 模式。不依赖 Gulp/Vinyl 的新代码继续使用主入口。

新代码或较大的迁移更推荐 `optimize(config)`。配置对象更容易测试、序列化、缓存，也更容易和 CLI 配置文件共享：

```ts
import { css, glyph, optimize, ttf2woff2 } from 'fontmin-rs'

await optimize({
  input: ['fonts/roboto.ttf'],
  outDir: 'build',
  runtime: 'auto',
  cache: { enabled: true },
  plugins: [
    glyph({ text: 'Hello' }),
    ttf2woff2(),
    css({ fontFamily: 'Roboto', fontPath: './' }),
  ],
})
```

## 插件映射

| Fontmin 风格操作     | `fontmin-rs` API                         | 说明                                                                                                        |
| -------------------- | ---------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| `glyph(options)`     | `glyph(options)`                         | 支持 text、text file、Unicode 列表和布局保留模式。                                                          |
| `ttf2woff(options)`  | `ttf2woff(options)` / `ttfToWoff()`      | 低层 API 支持 WOFF metadata 和 private data。                                                               |
| `ttf2woff2(options)` | `ttf2woff2(options)` / `ttfToWoff2()`    | Pipeline 支持 `native`、`wasm` 和 `auto`；省略 `runtime` 时，旧 plugin `fallback` 会选择 pipeline runtime。 |
| `ttf2eot(options)`   | `ttf2eot(options)` / `ttfToEot()`        | 用于旧版 IE 兼容。                                                                                          |
| `ttf2svg(options)`   | `ttf2svg(options)` / `ttfToSvg()`        | 输出 SVG font。                                                                                             |
| `svg2ttf(options)`   | `svg2ttf(options)` / `svgFontToTtf()`    | 将 SVG font 转为 TTF。                                                                                      |
| `svgs2ttf(file, options)` | `svgs2ttf(file, options)` / `svgs2ttf(options)` / `svgsToTtf()` | 将多个 SVG icon 合并为一个 TTF iconfont；同时支持经典输出文件重载和仅 options 形式。 |
| `css(options)`       | `css(options)` / `generateFontFaceCss()` | 支持 CSS、SCSS、Less target 和可选 glyph class。                                                            |

如果希望快速得到一组 Fontmin 风格产物，可以使用 `fontminCompatPreset(options)`：

```ts
import { fontminCompatPreset, optimize } from 'fontmin-rs'

await optimize({
  input: ['fonts/roboto.ttf'],
  outDir: 'build',
  plugins: fontminCompatPreset({
    text: 'Hello',
    fontFamily: 'Roboto',
    fontPath: './',
  }),
})
```

如果只需要现代 Web 输出，使用 `modernWeb(options)`。它会输出 WOFF2、WOFF 和 CSS，不会输出 EOT 或 SVG。

## CLI 替换

很多 Fontmin 构建脚本可以先迁移到 CLI：

```sh
fontmin-rs build fonts/roboto.ttf \
  --out-dir build \
  --text "Hello" \
  --preset compat \
  --font-family Roboto \
  --font-path ./
```

使用 `--preset modern-web` 输出 WOFF2、WOFF 和 CSS。多个 SVG icon 输入可以使用 `--preset iconfont`：

```sh
fontmin-rs build icons/home.svg icons/user.svg \
  --out-dir build/icons \
  --preset iconfont \
  --font-family "Project Icons"
```

Iconfont preset 不支持 delivery slices。

## 配置文件

可以把重复 CLI 参数放入 `fontmin.config.jsonc`：

```jsonc
{
  "input": ["fonts/roboto.ttf"],
  "outDir": "build",
  "clean": true,
  "subset": {
    "text": "Hello",
    "basicText": true,
  },
  "outputs": [{ "format": "woff2" }, { "format": "woff" }, { "format": "css" }],
  "css": {
    "fontFamily": "Roboto",
    "fontPath": "./",
    "fontDisplay": "swap",
  },
  "cache": {
    "enabled": true,
  },
}
```

然后运行：

```sh
fontmin-rs build --config fontmin.config.jsonc
```

## 行为差异

- 主兼容链输出类型化 `FontAsset`。已有构建依赖真实 Vinyl 文件、`vinyl-fs` 选项或 Transform 插件时，使用独立的 `fontmin-rs/vinyl` 入口；新代码更推荐 `runAsync()` 和 `optimize(config)`。
- `definePlugin()` 创建的插件收到 typed asset 和 context；传给 `fontmin-rs/vinyl` 的插件也可以是普通 Vinyl Transform stream。即使内置操作使用 WASM，两种适配器和所有文件 I/O 仍在 Node 端运行。
- Rust plugin 应通过 `AssetMeta.unicode`、`AssetMeta.css_glyphs` 和 `AssetMeta.css_unicode_ranges` 设置内置 plugin 会消费的元信息；`AssetMeta.custom` 继续作为第三方 key 的扩展 map。
- 当前支持 OTF inspect。`otf2ttf()` / `otfToTtf()` 可以将静态 CFF OTF 以及 CFF2 默认/显式实例转换为静态 TrueType `glyf` 字体，也可以将 glyf-backed OTF wrapper 重写为 TTF；静态输出会移除 CFF2 和 variation 表。
- `optimize({ runtime })` 为所有内置操作选择一个 runtime：`native` 是默认值，`wasm` 强制使用 WASM，`auto` 只在 native binding 无法加载时回退。转换错误不会触发 WASM 重试。
- 对于旧 `ttf2woff2({ fallback })` plugin，省略 pipeline `runtime` 时会继承 `native`、`wasm` 或 `auto`；匹配的显式 runtime 可以共存，不同 runtime 或多个不同 plugin fallback 会冲突，`js` 仍不受支持。低层 `ttfToWoff2Async(input, { fallback: 'wasm' | 'auto' })` 仍可独立使用。
- native 包是平台相关 optional dependencies。安装异常时，可以删除 `node_modules` 和对应包管理器 lockfile 后重新安装。

## 验证清单

1. 对比生成文件名和扩展名。
2. 检查 CSS 中的 `font-family`、`font-path` 和 `font-display` 输出。
3. 对生成字体运行 `fontmin-rs inspect <font> --json`。
4. 在应用或 browser test 中加载生成的 WOFF2/WOFF/CSS。
5. 确认未启用 cache 时产物正确，再启用 cache。
