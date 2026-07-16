---
layout: home

hero:
  name: fontmin-rs
  text: 高性能字体压缩与转换工具链
  tagline: 用 Rust 处理字体核心逻辑，覆盖 CLI、Node.js 与浏览器 WASM 工作流。
  image:
    src: /logo.svg
    alt: fontmin-rs
  actions:
    - theme: brand
      text: 快速开始
      link: /zh/guide/getting-started
    - theme: alt
      text: Node API
      link: /zh/api/node

features:
  - title: 字体子集化
    details: 按 text、textFile、unicodes 或 basicText 保留实际需要的字形，并支持 conservative/preserve 等布局保留策略。
  - title: 多格式输出
    details: 当前支持 TTF、WOFF、WOFF2、EOT、SVG font 和 @font-face CSS 输出，适合现代 Web 与遗留兼容场景。
  - title: Native + WASM
    details: 可使用 Rust CLI、在 Node.js 中选择 native 或 WASM，也可在浏览器本地运行纯内存 WASM 包。
---

## 当前状态

fontmin-rs 仍处在早期实现阶段，但已经具备一条可用的字体处理链路：

- `fontmin-rs subset` 按文本裁剪 TTF。
- `fontmin-rs convert` 在 TTF、WOFF、WOFF2、EOT、SVG font 等格式间转换。
- `fontmin-rs build` 从输入字体批量生成多格式产物和 CSS。
- `fontmin-rs inspect` 输出 TTF、OTF、WOFF、EOT 等字体元信息。
- `fontmin-rs init` 创建初始 JSONC 配置文件。
- `fontmin-rs` npm 包提供同名 bin、低层 native helpers、`optimize(config)` pipeline 和 Fontmin-compatible chain。
- `@fontmin-rs/wasm` 包提供异步直接 API，以及面向浏览器的纯内存 `optimizeBrowser(config)` pipeline。

## 安装

::: code-group

```sh [pnpm]
pnpm add fontmin-rs
```

```sh [npm]
npm install fontmin-rs
```

```sh [yarn]
yarn add fontmin-rs
```

:::

## 最短示例

```sh
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf \
  -o build \
  --text "Hello" \
  --formats woff2,woff,css \
  --font-family Roboto
```

继续阅读 [快速开始](/zh/guide/getting-started)，了解 CLI、配置文件与 Node API 的组合用法。如果你已经在使用 Fontmin，可以从[迁移指南](/zh/guide/migration)开始。
