# 从 Fontmin 迁移

`fontmin-rs` 保留 Fontmin 主流工作流，同时把耗时字体操作下沉到 Rust 和 N-API binding。这一页适合已经在构建脚本里使用 `fontmin`，并希望逐步迁移的项目。

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
| `svgs2ttf(options)`  | `svgs2ttf(options)` / `svgsToTtf()`      | 将多个 SVG icon 合并为一个 TTF iconfont。                                                                   |
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

- 兼容链支持常见 Fontmin 风格用法，但不是 Node stream 的完整克隆。新代码更推荐 `runAsync()` 和 `optimize(config)`。
- 自定义 JavaScript 插件收到的是 typed asset 和 context 对象，而不是 vinyl stream。即使内置操作使用 WASM，它们和所有文件 I/O 仍在 Node 端运行。
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
