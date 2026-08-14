# Fontmin 功能覆盖、兼容差距与同类工具扩展调研

## 结论摘要

- 调研基线已经提供与原版九个内置插件对应的结果能力（9/9），并拥有 Rust、WASM、缓存和现代交付能力，但当时尚不是 `fontmin@1.1.1` 的完整 drop-in 替代。
- 已确认的兼容差距集中在默认多格式管线、`run()` 返回 stream、静态辅助导出、getter、插件签名/默认值/语义以及 Vinyl Transform 协议。
- 原版 Fontmin 的核心基准是 Vinyl 管线和九个内置插件；原作者个人名下的旧扩展不应计入核心兼容承诺。
- 扩展优先级建议依次为：补齐兼容层、开放专业子集控制、变量字体实例化、网页字符发现与交付自动化，再考虑集合/颜色字体和低层映射 API。

## 落地状态（2026-08-14）

本文的事实章节保留实施前的固定基线，便于复核为什么做这些取舍；其中“当前仓库”的描述
不应再作为实施后的能力清单。沿本文推荐路线，本轮已完成：

- Fontmin 兼容默认管线、stream/getter/static helper、经典默认值与回调，以及独立 Vinyl adapter；
- 跨 Rust/CLI/Node/WASM 的 GID、glyph name、layout/name/cmap/table 策略、缺字策略与详细映射报告；
- variable font 全轴实例化、单轴 pin 与范围收窄，以及可选的完整 CFF2→CFF1 降级；
- 本地网页源码字符发现、CSS/manifest/preload/fallback、按压缩体积约束的语言/CJK 自动分片、
  hash 产物、报告和预览 HTML；
- TTC/OTC face 检查与提取、彩色字体结构化 capability report，以及 SVG smooth path、arc 和
  补充平面 Unicode；
- 带 schema、计划/源字体 SHA-256、覆盖率、selector mapping 和 seed GID 的可序列化 subset plan，
  以及返回实际表、体积和 GID mapping 的执行结果。

仍保留为明确边界、未纳入本轮稳定接口的竞品能力包括：联网/递归/headless browser 爬取与
CSS font-family 归因、保持 CFF/CFF2 outline 的直接子集、CFF desubroutinize、IUP delta
optimization、WOFF Zopfli、通用逐 glyph SVG/HTML dump，以及仍属实验性的 IFTB。它们适合在
独立适配层、明确格式后端或实验 feature 中继续推进，不应被本轮能力表误报为已支持。

## 调研范围与版本口径

- 调研日期：2026-08-14。
- 本仓库口径：当前工作树 `fontmin-rs@1.0.2-rc.1`。
- 资料只取自项目自己的仓库、源码、官方文档和 npm 注册表元数据；不使用第三方评测或二手文章。
- 本仓库已有 `docs/superpowers/research/` 的日期前缀惯例，因此本文放在该目录，不进入公开文档站导航。
- “事实”章节只描述可由固定源码或官方文档验证的行为；“建议”章节才包含对 `fontmin-rs` 的取舍判断。

最容易混淆的是 Fontmin 的版本：上游 GitHub `master` 固定提交
[`595460b`](https://github.com/ecomfe/fontmin/tree/595460b10e07de31043e777775985a9c1684f85b)
声明版本 **2.0.3**，是 ESM 包并要求 Node.js 16+；但 npm 的 `latest` dist-tag
在本次调研时指向 **1.1.1**，且本仓库也以 `fontmin@1.1.1` 作为兼容基准
（[`packages/fontmin/package.json`](../../../packages/fontmin/package.json) 与
[`pnpm-lock.yaml`](../../../pnpm-lock.yaml)）。因此本文同时给出：

1. 以 **1.1.1** 判断本仓库承诺的实际 npm 兼容口径；
2. 以 GitHub **2.0.3** 固定源码枚举上游现存的完整 API 和插件能力。

上游 2.x 的 ESM/Node 16 说明见
[`README.md`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/README.md#L24-L36)，
版本和运行时声明见
[`package.json`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/package.json#L1-L37)；
npm 的发布时间分别为 2.0.3（2025-08-01）和 1.1.1（2025-08-13），以
[`registry.npmjs.org/fontmin`](https://registry.npmjs.org/fontmin) 的 `time` 与
`dist-tags` 字段为准。

### 同类工具固定基准

| 工具 | 本次固定版本 / 提交 | 提交日期 | 本文关注点 |
| --- | --- | --- | --- |
| fontTools / `pyftsubset` | `4.63.1.dev0` / [`4e89641`](https://github.com/fonttools/fonttools/tree/4e896414b1ae147e1b68229b810d0eeda4f1e179) | 2026-08-12 | OpenType 子集控制面、表和元数据策略、CLI/Python API |
| glyphhanger | `6.0.0` / [`8b81f57`](https://github.com/zachleat/glyphhanger/tree/8b81f57dc214cb2a1d1b5abb209e1d39454d7c51) | 2026-06-08 | 网页字符发现、爬取、CSS 选择器和字体族归因 |
| subfont | `7.2.3` / [`19ad37c`](https://github.com/Munter/subfont/tree/19ad37c531a4b5f301003dcd1d5af7e4cae03b6c) | 2026-03-21 | 站点资产图、字体交付优化、变量字体实例化 |
| subset-font | `2.5.0` / [`74ee885`](https://github.com/papandreou/subset-font/tree/74ee885bc50c862b0a238e4dc32a74e2719cfe71) | 2026-04-02 | Node Buffer API、HarfBuzz WASM、变量轴裁剪 |
| HarfBuzz | `14.3.1` / [`dfdc088`](https://github.com/harfbuzz/harfbuzz/tree/dfdc088c4d7c5d31dd5b13070b919b51f6c21ea8) | 2026-08-13 | 底层子集语义、变量轴、颜色/位图表、映射与计划 API |
| cn-font-split | `7.4.3` / [`1650af1`](https://github.com/KonghaYao/cn-font-split/tree/1650af130b8c547b13a883ac7546f088ff193bf5) | 本次查询 | CJK 自动分包、请求数优化、预览/报告和前端构建集成 |

版本值分别来自各固定提交中的
[`fontTools.__version__`](https://github.com/fonttools/fonttools/blob/4e896414b1ae147e1b68229b810d0eeda4f1e179/Lib/fontTools/__init__.py#L1-L8)、
[`glyphhanger/package.json`](https://github.com/zachleat/glyphhanger/blob/8b81f57dc214cb2a1d1b5abb209e1d39454d7c51/package.json#L1-L12)、
[`subfont/package.json`](https://github.com/Munter/subfont/blob/19ad37c531a4b5f301003dcd1d5af7e4cae03b6c/package.json#L1-L7)、
[`subset-font/package.json`](https://github.com/papandreou/subset-font/blob/74ee885bc50c862b0a238e4dc32a74e2719cfe71/package.json#L1-L10)
和 [`harfbuzz/meson.build`](https://github.com/harfbuzz/harfbuzz/blob/dfdc088c4d7c5d31dd5b13070b919b51f6c21ea8/meson.build#L1-L12)。
cn-font-split 版本来自固定提交的
[`packages/ffi-js/package.json`](https://github.com/KonghaYao/cn-font-split/blob/1650af130b8c547b13a883ac7546f088ff193bf5/packages/ffi-js/package.json#L1-L6)。

## 事实：原版 Fontmin 的完整功能基准

### 核心 API 与执行模型

Fontmin 是基于 Vinyl 文件对象和 Node Transform stream 的插件管线。固定源码公开的能力如下：

| API | 上游事实 |
| --- | --- |
| `new Fontmin()` | 创建同时继承 `EventEmitter` 的管线实例。 |
| `.src(file)` | 接受 Buffer、glob 字符串或 glob 数组；无参数时是 getter。源码还会把第二个参数传给 `vinyl-fs.src`，但 README 和类型声明没有记录该重载。 |
| `.dest(dir)` | 设置输出目录；无参数时是 getter；未设置时不写文件。 |
| `.use(plugin)` | 接受 Vinyl Transform，或调用后返回 Transform 的工厂函数，并按加入顺序串联。 |
| `.run(cb)` | 启动管线，回调接收 Vinyl 文件数组，并返回可读/可写 stream。README 写作 `cb(err, files, stream)`，实现只向回调传 `err, files`，stream 是返回值。 |
| `.runAsync()` | Promise 版本，源码和类型声明存在，但 README 未记录。 |
| 静态/辅助导出 | `Fontmin.plugins` 列出九个内置插件；同时导出 `util` 和 `mime`。`util` 包含系统字体目录、纯文本整理、basic text、字符串到 Unicode code point 等函数。 |

来源：核心实现
[`index.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/index.js#L28-L205)、
类型声明
[`index.d.ts`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/index.d.ts#L80-L175)
和辅助函数
[`lib/util.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/lib/util.js#L13-L123)。

若用户没有调用 `.use()`，Fontmin 会自动建立
`OTF→TTF → TTF→EOT → WOFF → WOFF2 → SVG Font → CSS` 管线；这条默认管线不包含
`glyph` 子集化。只要用户加入了任何插件，便只运行显式插件栈。
来源：[`createStream()`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/index.js#L127-L152)。

### 九个内置插件

上游 README 和 `Fontmin.plugins` 都列出以下九项，故这是“官方内置”而不是社区扩展。

| 插件 | 输入 → 输出 | 主要能力与选项 |
| --- | --- | --- |
| `glyph` | TTF → 子集 TTF | 按 `text` 的 Unicode code point 取字形并保留第 0 glyph；`basicText` 追加基本字符，`hinting` 默认 `true`，`trim` 默认 `true`，`use(ttf)` 可对 `fonteditor-core` TTF 对象做二次处理。空子集不会删除任何字形。 |
| `otf2ttf` | OTF/CFF → TTF | CFF outline 转 TrueType outline；`clone=false`、`hinting=true`，也读取 `text/basicText` 交给底层转换。 |
| `ttf2eot` | TTF → EOT | 默认 `clone=true`，因此保留输入 TTF 并追加 EOT。 |
| `ttf2woff` | TTF → WOFF | 默认 `clone=true`；`deflate` 默认 `false`，启用时用 pako deflate。源码说明某些 Android 设备存在兼容顾虑。 |
| `ttf2woff2` | TTF → WOFF2 | 默认 `clone=true`，使用 `ttf2woff2` 依赖。 |
| `ttf2svg` | TTF → SVG Font | 输出单个 SVG Font，而不是每个 glyph 一份 SVG；默认 `clone=true`。 |
| `css` | TTF → TTF + CSS | 生成引用 EOT、WOFF2、WOFF、TTF、SVG 的 `@font-face`；支持 `fontPath`、TTF `base64`、逐 glyph class、`iconPrefix`、字符串或函数形式的 `fontFamily`、`asFileName`、`local`，源码另支持 `filename`。 |
| `svg2ttf` | SVG Font → TTF | `clone=true`、`hinting=true`。 |
| `svgs2ttf` | 多个独立 SVG → iconfont TTF | 必须给输出文件或 Vinyl 文件；支持 `fontName`、`startCode`、名称表和 glyph 对齐参数；未带 Unicode 的 SVG 从 `U+E001` 起自动分配私用区码点。 |

插件清单和公开示例见
[`README.md`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/README.md#L105-L257)；
更完整的选项来自
[`glyph.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/plugins/glyph.js#L15-L167)、
[`otf2ttf.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/plugins/otf2ttf.js#L16-L85)、
[`ttf2woff.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/plugins/ttf2woff.js#L16-L96)、
[`css.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/plugins/css.js#L33-L208)、
[`font-face.tpl`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/lib/font-face.tpl#L1-L33)、
[`svg2ttf.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/plugins/svg2ttf.js#L15-L80)
和 [`svgs2ttf.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/plugins/svgs2ttf.js#L17-L223)。

### 格式矩阵

| 格式/产物 | 上游作为输入 | 上游作为输出 | 边界 |
| --- | --- | --- | --- |
| TTF | 是 | 是 | `glyph` 的直接子集格式，也是其他转换的枢纽。 |
| OTF/CFF | 是 | 否 | 先由 `otf2ttf` 转成 TrueType；`glyph` 不直接处理 OTF。上游 TODO 仍把直接 OTF glyph 支持列为未完成。 |
| EOT | 否 | 是 | 只有 TTF→EOT。 |
| WOFF | 否 | 是 | 只有 TTF→WOFF，可选 deflate。 |
| WOFF2 | 否 | 是 | 只有 TTF→WOFF2。 |
| SVG Font | 是 | 是 | `svg2ttf` 读取，`ttf2svg` 写出。 |
| 独立 SVG 图形集合 | 是 | 间接 | `svgs2ttf` 合并为 TTF iconfont。 |
| CSS | 否 | 是 | `@font-face` 和可选 icon class/base64。 |
| TTC/OTC | 否 | 否 | 内置插件没有集合字体选择或输出接口。 |

这里的“否”来自内置插件实际输入检测与转换方向，不是 MIME 表推断；`mime` 中出现格式名
不等于可以解析该格式。直接 OTF 子集限制见
[`TODO.md`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/TODO.md#L15-L23)。

### CLI

CLI 接受文件、目录、glob 或 stdin；TTY 下默认写 `build`，重定向时向 stdout 写所有产物。
固定执行 `otf2ttf → glyph → ttf2eot/ttf2svg/ttf2woff/ttf2woff2 → css`，暴露的参数只有
`--text`、`--basic-text`、`--deflate-woff`、`--font-family`、`--css-glyph` 和
`--show-time`。它没有暴露 SVG→TTF、多个 SVG 合并、CSS base64、hinting 等完整库选项。
来源：[`cli.js`](https://github.com/ecomfe/fontmin/blob/595460b10e07de31043e777775985a9c1684f85b/cli.js#L17-L142)。

### 原作者名下的关联扩展（不是九个官方内置插件）

这些仓库可用于理解旧生态的能力边界，但不应计入核心包的默认承诺，也不宜因名字相似就称为当前官方插件。

| 扩展 | 固定版本 / 最后提交 | 事实能力 |
| --- | --- | --- |
| `fontmin-concat` | `0.0.2` / [`cac8664`](https://github.com/junmer/fontmin-concat/tree/cac866491a68a15e52c54684a1d709c20332080c)，2016-02-16 | 合并多个 TTF；后来的字体去掉与主字体重复的 Unicode glyph，并支持字体名、hinting、optimize 和位置调整。 |
| `fontmin-dump` | `0.0.1` / [`c60598b`](https://github.com/junmer/fontmin-dump/tree/c60598b98a8c458bf90d4ec8134b8559f015cd87)，2016-04-08 | 从 TTF 输出原字体、glyph JSON 和 HTML 预览。 |
| `fontmin-ttf2svgs` | `0.0.2` / [`bd72406`](https://github.com/junmer/fontmin-ttf2svgs/tree/bd7240667893693e6ca3299aa15dbe29df80f9a6)，2016-04-09 | 把简单 Unicode glyph 拆成独立 SVG 并生成 JSON 映射；支持尺寸、填色、命名与排除函数。 |
| `fontmin-typeface` | `0.0.1` / [`754cca9`](https://github.com/junmer/fontmin-typeface/tree/754cca9c333683dd92411dc4741f29bdc82b73fb)，2016-11-23 | TTF 转 three.js Typeface JSON/JS。 |
| `fontmin-wawoff2` | `0.0.2` / [`0ca23f5`](https://github.com/junmer/fontmin-wawoff2/tree/0ca23f563b3d36fe2f7ac3c5c0d7ca19cdc43d2d)，2018-03-08 | 基于 `wawoff2` 的另一套 TTF→WOFF2 实现；核心包后来已有 WOFF2 插件。 |
| `fontmin-otf2ttf` | `0.0.0` / [`1ed87a6`](https://github.com/junmer/fontmin-otf2ttf/tree/1ed87a69dde7871940577ce57e0d703dc303e902)，2015-02-13 | 早期实验性 OTF→TTF；README 明示尚不可用，当前应以核心内置实现为准。 |

## 事实：本仓库已经覆盖和领先的能力

若只比较“能否产出对应结果”，本仓库已为原版九个内置插件提供同名操作：`glyph`、
`otf2ttf`、`ttf2eot`、`ttf2woff`、`ttf2woff2`、`ttf2svg`、`css`、`svg2ttf` 和
`svgs2ttf`。这也是把覆盖结论写作“结果能力 9/9、drop-in 兼容未完成”的原因。完整清单见
[`docs/guide/features.md`](../../guide/features.md) 和
[`packages/fontmin/src/plugins.ts`](../../../packages/fontmin/src/plugins.ts)。

相对原版 Fontmin，当前仓库已有这些明确增量：

| 方向 | 本仓库现有增量 |
| --- | --- |
| 运行时 | 同一组核心操作覆盖 Rust CLI、Node 原生绑定和浏览器内存型 WASM；Node 管线还能显式选择 `native`、`wasm` 或 `auto`。 |
| 格式 | 除原版单向输出外，还提供 WOFF/WOFF2/EOT→TTF；可 inspect TTF、OTF、WOFF、WOFF2、EOT；能把静态 CFF OTF 或指定 CFF2 实例转换为静态 `glyf` TTF。 |
| 子集语义 | 除文本外支持文本文件、code point、Unicode range、缺字 `ignore/warn/error`、`.notdef`、hinting，以及 layout `drop/conservative/preserve`。 |
| Web 交付 | 有 `modernWeb`、配置化输出、CSS/SCSS/Less、`font-display`、Base64、`deliverySlices` 与 `unicode-range`。 |
| 工程化 | 类型化资产和插件生命周期、JSON/JSONC/JS/TS 配置、缓存、诊断码、coverage/inspect/doctor/bench 命令。 |

当前提交的本地验证结果为：

- `pnpm --filter fontmin-rs test -- --run`：13 个测试文件、201 个测试通过；
- `cargo test --locked -p fontmin_subset -p fontmin_otf -p fontmin_svg -p fontmin`：109 个测试通过；
- 提交的 release-profile 基准中，`glyph + ttf2woff` 为 2.2743 ms，经典 Fontmin 为
  12.4341 ms，约快 5.47 倍。这里的对照依赖是本仓库锁定的 `fontmin@1.1.1`，不能外推为
  对 GitHub 2.0.3 或所有字体都快 5.47 倍。证据见
  [`docs/benchmarks.md`](../../benchmarks.md) 与
  [`benchmarks/baseline.json`](../../../benchmarks/baseline.json)。

## 事实：本仓库相对 `fontmin@1.1.1` 的已确认兼容差距

以下只列“同名 API 的行为不兼容”；本仓库新增的 Rust CLI、WASM、缓存、现代交付切片等能力
不因这些差距而失效。

| 差距 | 本仓库证据 | 上游基准 |
| --- | --- | --- |
| 无插件默认管线 | `FontminCompat.config()` 原样传空 `plugins`，而 outputs 未配置时也不补插件，因此 `new Fontmin().src(...).dest(...).run()` 不会生成原版的 TTF/EOT/WOFF/WOFF2/SVG/CSS 包。 | 上游 `createStream()` 在没有显式插件时补六个转换/生成插件。 |
| `run()` 返回值 | 本仓库 `run(callback): void`；原版返回 stream。 | [`compat.ts`](../../../packages/fontmin/src/compat.ts) 对比上游 `index.js`。 |
| 静态与辅助导出 | 本仓库兼容类有各插件静态方法，但没有 `Fontmin.plugins`、`Fontmin.util`、`Fontmin.mime`。 | 上游 `index.js` 第 170–205 行。 |
| `src()` / `dest()` getter | 本仓库方法必须传参且只作 setter；不支持原版无参数 getter，也不支持上游源码中额外的 Vinyl FS options 参数。 | [`compat.ts`](../../../packages/fontmin/src/compat.ts)。 |
| `svgs2ttf` 调用形态 | 本仓库只接受 `svgs2ttf(options)`；原版是 `svgs2ttf(file, opts)`，输出文件是必需参数。 | [`plugins.ts`](../../../packages/fontmin/src/plugins.ts) 对比上游 `svgs2ttf.js`。 |
| `glyph` 默认 hinting | 本仓库 `preserveHinting`/`hinting` 都未给出时取 `false`；原版默认 `hinting=true`。 | [`plugins.ts`](../../../packages/fontmin/src/plugins.ts) 第 21–37 行对比上游 `glyph.js` 第 116–123 行。 |
| `glyph.use(ttf)` | 本仓库 `SubsetOptions` 和内置描述符没有原版的 TTF 对象回调。 | [`plugins.ts`](../../../packages/fontmin/src/plugins.ts) 与上游 `glyph.js` 第 40–66、105–123 行。 |
| 空 `glyph()` 请求 | 本仓库没有 text/unicode/range 时会报配置错误；原版空选择不会删除 glyph，仍会重写/返回字体。 | [`crates/fontmin_subset/src/lib.rs`](../../../crates/fontmin_subset/src/lib.rs) 的 `collect_requested()` 对比上游 `glyph.js`。 |
| CSS `local` 默认值 | 本仓库 Rust CSS 默认 `local=true`；原版默认 `false`。 | [`crates/fontmin_css/src/lib.rs`](../../../crates/fontmin_css/src/lib.rs) 第 27–40 行对比上游 `css.js` 第 147–159 行。 |
| CSS `asFileName` 语义 | 原版用它强制把 `font-family` 改成文件名；本仓库用它决定 glyph class 采用 glyph 名还是 Unicode fallback，并未执行同名语义。 | [`crates/fontmin_css/src/lib.rs`](../../../crates/fontmin_css/src/lib.rs) 第 247–263 行对比上游 `css.js` 第 173–179 行。 |
| CSS `fontFamily` 回调 | 本仓库回调只接收 `FontInfo`；原版还传入可修改的 TTF 对象。 | [`packages/fontmin/src/types.ts`](../../../packages/fontmin/src/types.ts) 对比上游 `css.js`。 |
| 自定义插件协议 | 本仓库插件是带 `name` 及生命周期钩子的对象；原版 `.use()` 接受 Vinyl Transform/工厂。现有接口更适合 Rust/WASM 管线，但不是 drop-in 兼容。 | [`types.ts`](../../../packages/fontmin/src/types.ts) 第 64–91 行对比上游 `index.js` 第 81–91 行。 |
| Node.js 门槛 | 本仓库 npm 包要求 Node.js `>=22.18.0`；上游 1.1.1 要求 `>=12`，GitHub 2.0.3 要求 `>=16`。 | [`packages/fontmin/package.json`](../../../packages/fontmin/package.json) 与上游 npm/GitHub 元数据。 |

因此，若“覆盖 Fontmin 全部功能”指 **结果能力**，本仓库已有很多等价或更现代的实现；若指
`fontmin@1.1.1` 的 **drop-in API/默认行为兼容**，当前答案是 **不能完全覆盖**，上表各项都需要
兼容层修正或在文档中明确列为不兼容。

## 事实：超出 Fontmin 基准后的字体能力边界

如果目标不是复刻 Fontmin，而是做通用的现代字体工具，当前还存在这些较重要的技术边界：

- 公开子集入口仍以 TTF 为中心；管线里的 `glyph` 只处理 `asset.format === "ttf"`。OTF/CFF/CFF2
  需要先转成静态 `glyf` TTF，尚无保持 CFF/CFF2 outline 的直接子集；也没有 TTC/OTC face
  选择。证据见 [`optimize-transforms.ts`](../../../packages/fontmin/src/optimize-transforms.ts) 和
  [`crates/fontmin/src/lib.rs`](../../../crates/fontmin/src/lib.rs)。
- 公开配置没有 GID/glyph name、layout feature/script/language、retain-GID、name ID/language、
  drop/pass-through table、old/new GID mapping 或 subset stats；这些正是 fontTools/HarfBuzz 的
  专业控制面。
- 当前能保留 `glyf` variable font 的 `fvar/gvar` 等表，也能把 CFF2 全量实例化为静态字体；
  但还没有“固定部分轴”或“收窄轴 min/default/max 范围”的部分实例化 API。
- CFF/CFF2→TTF 会拒绝 COLR/CPAL、CBDT/CBLC、`sbix`、SVG 色彩表，且静态输出移除 variation
  tables、不能保留 Type 2 hinting。证据见
  [`crates/fontmin_otf/src/sfnt.rs`](../../../crates/fontmin_otf/src/sfnt.rs) 和
  [`docs/guide/features.md`](../../guide/features.md)。
- vendored 子集引擎能改写 COLR v0、SVG、`sbix`、CBDT/CBLC，但 COLR v1+ 仍原样透传；当 GID
  重排时不能把它视为已完整验证的 COLR v1 子集。当前生产语料对 Noto Color Emoji 做的是
  inspect，而不是子集后的 shape/render 验证。证据见
  [`vendor/oxifont-subset/src/colr.rs`](../../../vendor/oxifont-subset/src/colr.rs) 与
  [`docs/benchmarks.md`](../../benchmarks.md)。
- SVG icon path 只实现 M/L/H/V/Q/C/Z，S/T/A 等命令会报不支持，曲线按固定步数折线化；
  SVG Font 和 iconfont 写出目前只接受 BMP code point。证据见
  [`crates/fontmin_svg/src/icon/path.rs`](../../../crates/fontmin_svg/src/icon/path.rs) 和
  [`crates/fontmin_svg/src/icon.rs`](../../../crates/fontmin_svg/src/icon.rs)。
- Rust 配置中的 `parallel` 目前明确标为 reserved，并未形成可观察的并行管线能力。见
  [`docs/guide/config.md`](../../guide/config.md)。

## 事实：同类产品提供的额外能力

### fontTools / `pyftsubset`

`pyftsubset` 接受 TT/CFF flavored OTF/TTF 和 WOFF，既是 CLI 也是 Python API。初始集合可由
GID、PostScript glyph 名、文本、Unicode code point/range 及对应文件提供，并可分别控制缺失
glyph/Unicode 是忽略还是失败。来源：
[`fontTools.subset`](https://github.com/fonttools/fonttools/blob/4e896414b1ae147e1b68229b810d0eeda4f1e179/Lib/fontTools/subset/__init__.py#L30-L122)。

它比原版 Fontmin 的关键增量是：

- 输出 WOFF/WOFF2，可选 WOFF Zopfli，并可选择 HarfBuzz GPOS/GSUB repacker；
- `retain-gids`、`.notdef`/推荐 glyph 策略；
- OpenType layout closure，以及 feature、script、LangSys 的保留/排除集合；
- 去 hint、CFF desubroutinize；
- 精细的 drop/no-subset/passthrough table 策略；
- `name` ID/语言、legacy/symbol cmap、PS glyph name、legacy kern 和名称混淆；
- bounds、timestamp、Unicode/codepage range、平均字宽、max context 重算；
- 用 `--font-number` 选择 TTC/OTC 中的字体；可输出 TTX XML 和 timing；
- Python `Options`、`Subsetter.populate(glyphs, gids, unicodes, text)` 与 `subset(font)` API。

上述选项见固定源码的
[`输出与 glyph closure`](https://github.com/fonttools/fonttools/blob/4e896414b1ae147e1b68229b810d0eeda4f1e179/Lib/fontTools/subset/__init__.py#L141-L238)、
[`hint/表/名称`](https://github.com/fonttools/fonttools/blob/4e896414b1ae147e1b68229b810d0eeda4f1e179/Lib/fontTools/subset/__init__.py#L240-L387)、
[`重算与集合字体`](https://github.com/fonttools/fonttools/blob/4e896414b1ae147e1b68229b810d0eeda4f1e179/Lib/fontTools/subset/__init__.py#L389-L438)
和 [`Python API`](https://github.com/fonttools/fonttools/blob/4e896414b1ae147e1b68229b810d0eeda4f1e179/Lib/fontTools/subset/__init__.py#L3305-L3567)。

### glyphhanger

glyphhanger 在 `pyftsubset` 之上增加网页字符发现：可读本地 HTML/文本、stdin、单个或多个 URL；
可按 `font-family` 输出 JSON 或过滤字符，字符和 `unicode-range` 双向转换；支持 US-ASCII/Latin
预设和白名单。它还能站内 spider、只收集可见文字、限制 CSS selector，并在默认 headless browser
与更快的 JSDOM 模式之间选择。来源：
[`README 使用入口`](https://github.com/zachleat/glyphhanger/blob/8b81f57dc214cb2a1d1b5abb209e1d39454d7c51/README.md#L31-L70)、
[`白名单与爬取`](https://github.com/zachleat/glyphhanger/blob/8b81f57dc214cb2a1d1b5abb209e1d39454d7c51/README.md#L148-L194)
和 [`可见性/选择器/运行环境`](https://github.com/zachleat/glyphhanger/blob/8b81f57dc214cb2a1d1b5abb209e1d39454d7c51/README.md#L196-L228)。

它可直接调用 `pyftsubset` 输出 TTF/WOFF/WOFF2、报告前后体积，并生成带 `src` 与
`unicode-range` 的 `@font-face` CSS。来源：
[`README 子集与 CSS`](https://github.com/zachleat/glyphhanger/blob/8b81f57dc214cb2a1d1b5abb209e1d39454d7c51/README.md#L68-L146)
和 [`GlyphHangerSubset.js`](https://github.com/zachleat/glyphhanger/blob/8b81f57dc214cb2a1d1b5abb209e1d39454d7c51/src/GlyphHangerSubset.js#L13-L127)。

### subfont

subfont 把问题提升到站点交付层：静态分析页面与字体族的实际字符，报告字体缺字，生成精确子集，
支持本地字体和 Google Fonts；插入 preload，给子集重命名并把原字体留作动态内容 fallback，且把
原始 `@font-face` CSS 异步移出关键路径。它还能按实际使用缩减变量字体 variation space。
来源：[`README 功能清单`](https://github.com/Munter/subfont/blob/19ad37c531a4b5f301003dcd1d5af7e4cae03b6c/README.md#L7-L33)。

CLI 进一步支持 AssetGraph 输出、Browserlist 驱动格式、headless browser 动态跟踪、原地改写、
CSS inline、`font-display`、站内递归、相对 URL、dry-run 和 debug。
来源：[`README CLI`](https://github.com/Munter/subfont/blob/19ad37c531a4b5f301003dcd1d5af7e4cae03b6c/README.md#L63-L116)。

### cn-font-split

cn-font-split 更直接对标本仓库已有的 `deliverySlices`，但把“手工 Unicode range”推进到了
CJK 大字体的自动交付规划：它支持手工 subsets、语言区域聚合、超过目标大小后自动再切、保留
OpenType feature、合并过小切片以减少请求；还能指定目标 `chunkSize`、容差和最大分包数。
产物侧支持 hash 命名、CSS family/weight/style/display/local/compress、预览图、测试 HTML 和报告。
固定文档见
[`packages/ffi-js/README.md`](https://github.com/KonghaYao/cn-font-split/blob/1650af130b8c547b13a883ac7546f088ff193bf5/packages/ffi-js/README.md#L60-L132)。

它同时提供 Rust FFI/WASM，并宣称覆盖 Node.js、Bun、Deno、浏览器/WASI；关联的
`vite-plugin-font` 可接 Vite、Nuxt、Next、Webpack 和 Rspack。需要注意，其 7.x 迁移说明明确称
性能原因不再直接接受 WOFF2 作为分包输入，所以不能只看仓库描述就把所有格式都视为对等入口。
来源：[`项目 README`](https://github.com/KonghaYao/cn-font-split/blob/1650af130b8c547b13a883ac7546f088ff193bf5/README.md#L1-L53)
和 [`ffi-js README`](https://github.com/KonghaYao/cn-font-split/blob/1650af130b8c547b13a883ac7546f088ff193bf5/packages/ffi-js/README.md#L134-L207)。

### subset-font

subset-font 提供很小的 Node API：
`subsetFont(buffer, text, options): Promise<Buffer>`，输入 SFNT/WOFF/WOFF2，输出
`sfnt|woff|woff2`。它通过 HarfBuzz WASM 子集，并补上 `preserveNameIds`、
`noLayoutClosure` 和变量字体 `variationAxes`：既能把轴 pin 到一个值，也能收窄
`min/max/default` 范围。来源：
[`README 示例与 API`](https://github.com/papandreou/subset-font/blob/74ee885bc50c862b0a238e4dc32a74e2719cfe71/README.md#L1-L64)。

### HarfBuzz `libharfbuzz-subset` / `hb-subset`

HarfBuzz 当前明确支持 `glyf`、CFF、CFF2、`sbix`、COLR 和 CBDT/CBLC，包括 OpenType
variable outlines；layout 子集支持 GSUB/GPOS/GDEF。其官方说明同时明确：EBDT/EBLC、SVG，
以及 Graphite/AAT layout subsetting 仍不支持。来源：
[`hb-subset.cc`](https://github.com/harfbuzz/harfbuzz/blob/dfdc088c4d7c5d31dd5b13070b919b51f6c21ea8/src/hb-subset.cc#L43-L62)。

CLI 可按 GID、glyph 名、文本、Unicode 范围和文件做集合的加/减；还可配置 GID 映射、name ID/
语言、layout feature/script、drop table，以及变量字体部分或完整实例化。
来源：[`hb-subset CLI 集合`](https://github.com/harfbuzz/harfbuzz/blob/dfdc088c4d7c5d31dd5b13070b919b51f6c21ea8/util/hb-subset.cc#L885-L974)。

底层 flags 还覆盖去 hint、保留 GID、CFF/CFF2 desubroutinize、legacy name、overlap flag、
未知表透传、`.notdef` outline、glyph name、OS/2 range、layout/bidi closure、IUP delta 优化、
CFF2→CFF1 和 CID identity charset。输入 set 可独立控制 Unicode、GID、保留/删除表、name ID/
语言、layout feature/script。来源：
[`hb-subset.h flags 与 sets`](https://github.com/harfbuzz/harfbuzz/blob/dfdc088c4d7c5d31dd5b13070b919b51f6c21ea8/src/hb-subset.h#L58-L154)。

API 还支持 pin 全部/单个轴、设置轴范围、预处理复用、显式 subset plan，以及查询
old→new、new→old 和 Unicode→old GID 映射。
来源：[`轴 API`](https://github.com/harfbuzz/harfbuzz/blob/dfdc088c4d7c5d31dd5b13070b919b51f6c21ea8/src/hb-subset.h#L198-L238)
和 [`plan/mapping API`](https://github.com/harfbuzz/harfbuzz/blob/dfdc088c4d7c5d31dd5b13070b919b51f6c21ea8/src/hb-subset.h#L271-L309)。

## 建议：`fontmin-rs` 的扩展优先级

以下均为基于上述事实的建议，不代表上游已替本项目作出同样选择。

### P0：先完成兼容口径，再宣称“覆盖全部 Fontmin”

如果包继续提供 `FontminCompat`，建议优先修复前述兼容差距：默认多格式管线、`run()` stream、
静态 `plugins/util/mime`、getter、`svgs2ttf(file, opts)`、glyph hinting/`use`、CSS `local` 和
`asFileName`、Vinyl Transform 适配。也可以选择明确宣告“结果兼容而非 drop-in API 兼容”，
但必须把差异写入迁移文档并新增 1.1.1 行为对照测试。

其中 Vinyl Transform 兼容成本最高，建议放在独立 Node compatibility adapter 中，避免污染
Rust/WASM 原生插件生命周期；其余差距大多可在兼容类或兼容 preset 中封装。

### P0：开放专业子集控制面

建议从 fontTools/HarfBuzz 选取稳定的公共子集：

- 输入 GID、glyph name、Unicode ranges 和 text file；
- 缺字 `ignore|warn|error`；
- layout feature/script 与 layout closure；
- hinting、`.notdef`、retain GID；
- drop/pass-through tables；
- name ID/语言和 cmap/glyph-name 保留策略。

这些能力应落在统一的 Rust 配置模型，再投射到 CLI、Node 和 WASM；不要只做 CLI flags，否则
三个运行时会再次形成不同功能面。

### P0：变量字体部分/完整实例化

`subset-font` 和 HarfBuzz 都已把“单轴 pin + 轴范围 min/default/max”做成稳定用户能力。
建议为 `wght/wdth/slnt/opsz` 等任意 tag 提供统一轴配置，并在全轴固定时考虑可选 CFF2→CFF1。
这比继续增加传统 EOT/SVG Font 选项更能形成现代差异化。

### P1：网页感知的字符发现与交付产物

建议把 glyphhanger/subfont 的能力做成可选上层包或 CLI 子命令：

- 本地 HTML、URL、stdin、站内递归；
- 静态分析与 headless browser 两种模式；
- 可见性、CSS selector、`font-family` 归因；
- 生成 `unicode-range`、`@font-face`、preload、`font-display`；
- 保留原字体 fallback，以覆盖动态或用户生成内容；
- JSON manifest、前后体积和缺字诊断、dry-run。

浏览器爬取不应进入核心字体二进制模块；独立适配层能保持 `fontmin_core` 深而窄，也便于替换
浏览器实现。

### P1：把手工 delivery slices 升级为自动分包规划器

当前 `deliverySlices` 已经解决切片执行和 `unicode-range` 输出，下一步可借鉴 cn-font-split：

- 内置 script/language/frequency 预设，并允许业务字符频率作为输入；
- 以压缩后的目标字节数、容差、最大切片数和最大请求数为约束自动切分；
- 合并过小切片，避免“体积变小但网络请求爆炸”；
- 输出 hash 文件名、manifest、体积/覆盖率报告和可加载的测试 HTML；
- 对可变字体的不同 weight/style 共享规划，但分别验证实际表和 `unicode-range`。

这条路线能复用现有 Rust subset、delivery slice 元数据和 CSS 生成器，投入收益比新增一个孤立转换器
更高。

### P1：集合字体、颜色字体与能力探测

增加 TTC/OTC face 选择可直接补齐 Fontmin 和当前多数 Node 包的空白。对 COLR、CBDT/CBLC、
`sbix`、SVG 等表，建议输出结构化 capability report，并对“可子集、只能透传、不支持”分别处理；
HarfBuzz 当前自己也并非覆盖所有表，因此不应笼统宣传“所有 OpenType”。

### P2：可组合的低层结果与复用

面向 PDF、文档嵌入和多阶段增量生成，可增加：

- old/new GID mapping；
- 可缓存的 subset plan/preprocessed face；
- CFF desubroutinize、IUP delta optimization、WOFF Zopfli/压缩级别；
- JSON/HTML inspection 或逐 glyph SVG 导出，吸收旧 Fontmin 关联扩展中仍有价值的部分。

HarfBuzz 的 IFTB requirements 目前仍标为 experimental，建议只做实验 feature flag，不纳入稳定
跨运行时配置契约。

## 最终判断

1. **事实：当前仓库不能称为 `fontmin@1.1.1` 的完全 drop-in 覆盖。** 字体转换和现代能力很强，
   但默认产物、返回值、静态导出、若干插件签名/默认值/语义及自定义插件协议仍有明确差异。
2. **事实：原版 Fontmin 的完整核心范围是九个内置插件加 Vinyl 管线。** 旧作者扩展应单列，不能
   混作核心功能。
3. **建议：兼容层先补低成本行为差距；产品差异化优先做专业子集控制、变量字体实例化和网页感知
   交付。** EOT、SVG Font、旧 three.js Typeface 等主要属于兼容/长尾，不应压过现代方向。
4. **建议：对格式支持按表和字体类型给出能力矩阵。** `TTF/OTF/WOFF2` 的扩展名覆盖不等于
   `glyf/CFF/CFF2/COLR/CBDT/SVG/AAT/Graphite` 都能安全子集。
