# 快速开始

fontmin-rs 提供两个入口：

- 命令行工具 `fontmin-rs`，适合一次性处理、CI 和脚本。
- TypeScript API，适合嵌入构建流程或封装自己的字体流水线。

## 安装

```sh
pnpm add fontmin-rs
```

如果你在本仓库内开发，需要先构建调试版 native binding：

```sh
pnpm install
pnpm run build:debug
```

## 用 CLI 生成 Web 字体

```sh
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf \
  -o build \
  --text "Hello, fontmin-rs" \
  --preset modern-web \
  --font-family Roboto \
  --font-path ./
```

这条命令会：

1. 读取输入 TTF。
2. 仅保留 `--text` 中需要的字形。
3. 输出 WOFF2、WOFF 和 CSS。
4. 在 CSS 中生成可直接使用的 `@font-face`。

## 用 TypeScript API

```ts
import { modernWeb, optimize } from 'fontmin-rs'

await optimize({
  input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
  outDir: 'build',
  cache: { enabled: true },
  plugins: modernWeb({
    text: 'Hello, fontmin-rs',
    fontFamily: 'Roboto',
    fontPath: './',
    fontDisplay: 'swap',
  }),
})
```

`modernWeb()` 是一个预设插件组合，会执行子集化、生成 WOFF、生成 WOFF2，并输出 CSS。

## 在浏览器中使用

浏览器运行时由独立的 WASM 包提供，不依赖 Node.js 文件系统或原生 binding：

```sh
pnpm add @fontmin-rs/wasm
```

```ts
import { initWasm, modernWeb, optimizeBrowser } from '@fontmin-rs/wasm'

await initWasm()

const result = await optimizeBrowser({
  assets: [
    {
      contents: new Uint8Array(
        await (await fetch('/fonts/roboto.ttf')).arrayBuffer(),
      ),
      fileName: 'roboto.ttf',
    },
  ],
  plugins: modernWeb({ text: 'Hello', fontFamily: 'Roboto' }),
})
```

`optimizeBrowser()` 只接收和返回内存资产；由你的应用决定如何获取输入和写出结果。它支持子集化、WOFF/WOFF2/EOT/SVG 转换、OTF 与 SVG 输入，以及 CSS（包括 Base64 内嵌）生成。

## 检查字体元信息

```sh
fontmin-rs inspect fixtures/fonts/ttf/roboto-regular.ttf --json
```

Node API 中也可以直接调用：

```ts
import { inspect, validateWoff2 } from 'fontmin-rs'
import { readFileSync } from 'node:fs'

const input = readFileSync('fixtures/fonts/ttf/roboto-regular.ttf')
const info = inspect(input)

console.log(info.format)
console.log(info.metadata.familyName)

const woff2 = readFileSync('build/roboto-regular.woff2')
validateWoff2(woff2)
```

## 兼容性说明

EOT 输出用于旧版 IE 兼容场景。OTF 元信息检查已经可用，`otf2ttf` / `otfToTtf` 可以将静态 CFF OTF 转换为静态 TrueType `glyf` 字体，将 CFF2 可变字体实例化为默认或指定坐标的静态字体，也可以将 glyf-backed OTF wrapper 重写为 TTF。输出会移除 CFF2 和 variation 表，Type 2 hinting 会被丢弃。WOFF2 inspect/validate 会校验 header 和 table directory、读取 sfnt metadata tables，并通过 `woff2ToTtf()` 或 CLI `convert -f ttf` 支持 WOFF2 到 TTF decode。
