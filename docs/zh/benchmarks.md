# 性能策略

性能证据必须来自优化后的 native binding。Debug binding 适合开发和正确性检查，但不能
作为发布性能信号。

## 发布门禁

`pnpm run bench:report` 会先用 Cargo release profile 构建 native binding，再连续运行
三轮 Vitest benchmark，并把中位数报告写入 `benchmarks/current.json`。CI benchmark
任务固定 Ubuntu 24.04、Node.js 24 和仓库 Rust 工具链，使软件环境保持可比。

代表性兼容场景会在每一轮中，用同一份 Roboto 输入和 `glyph + ttf2woff` 请求分别运行
fontmin-rs 与经典 Fontmin。当 fontmin-rs 的配对平均耗时比例超过 1.10 时，发布门禁会
失败。同一进程内的配对比例比绝对毫秒阈值更不容易受到 hosted runner 硬件波动影响。

同一个 CI 任务会把固定提交的 production corpus 准备到
`fixtures/production/.cache`。它会分别通过 native 和 WASM 检查包含 31,036 个 glyph
的 Noto Sans SC variable font 与 Noto Color Emoji，并要求 Latin、CJK、标点混合
delivery slices 在两个运行时中保持逐字节一致。每个 variable-font slice 还必须是
非空子集，并继续保留 `fvar` 与 `gvar` 表。缓存 key 来自 production manifest 摘要；
下载内容在使用前仍会核对记录的字节数和 SHA-256。

本地运行完整 production conformance：

```sh
pnpm run fixtures:production:conformance
```

## Production 排版与渲染

`pnpm run fixtures:production:rendering` 还会用 `hb-shape` 和 `hb-view` 检查真实的
CID CFF、可变 CFF2 和 COLRv1 字体（需先安装 HarfBuzz）。检查在同一台机器上比较
原字体与 native、WASM 子集，先还原 GID 映射，再核对字形、cluster、advance 和
offset。五个样本覆盖中文、拉丁字距与连字、三个 CFF2 变量位置，以及彩色图层。
像素比较会明确拒绝空白输出和彩色字体退化成单色的情况。

CI 安装相关工具，并上传 `benchmarks/rendering-current/` 中的报告、原图和子集图，
失败时也会保留。报告记录工具版本和丢弃的 contextual 子表数量。这些检查验证的是
已声明的样本，不代表完整支持 contextual shaping 或 FeatureVariations。

## Production 耗时与内存预算

`pnpm run bench:production` 会先运行 conformance、构建 release CLI，再让每个
production stage 在独立进程中执行。每个 stage 采集三轮数据：耗时取中位数，避免一次
调度中断被误判为回退；内存则取最大的进程 `maxRSS`。Stage 隔离使失败能直接指出对应
runtime、操作和 fixture。

已提交的
[`benchmarks/production-budgets.json`](../../benchmarks/production-budgets.json)
定义 Ubuntu 24.04 与 Node.js 24 门禁：

| Stage 类别                               | 最大耗时中位数 | 最大 peak RSS |
| ---------------------------------------- | -------------: | ------------: |
| Native inspect                           |         500 ms |       128 MiB |
| WASM 初始化                              |         250 ms |       128 MiB |
| WASM inspect                             |         250 ms |       160 MiB |
| Native 混合 delivery                     |         500 ms |       192 MiB |
| Native 异步 WOFF2                        |      15,000 ms |       288 MiB |
| Native 四文件 subset                     |      20,000 ms |       512 MiB |
| Native 自动 delivery                     |      30,000 ms |       384 MiB |
| Native 缓存（512 条、并发 8）            |      20,000 ms |       160 MiB |
| Rust CLI 构建（10,000 个单字节输入）     |      90,000 ms |       256 MiB |
| Rust CLI 字体批处理（冷缓存、1 worker）  |      15,000 ms |       384 MiB |
| Rust CLI 字体批处理（冷缓存、4 workers） |      15,000 ms |       512 MiB |
| Rust CLI 字体批处理（热缓存、1 worker）  |       5,000 ms |       256 MiB |
| Rust CLI 字体批处理（热缓存、4 workers） |       5,000 ms |       384 MiB |
| WASM 混合 delivery                       |       1,000 ms |       256 MiB |

10,000 输入 stage 使用四个 CLI worker slot，并直接测量子进程。为容纳跨平台文件系统
开销，耗时上限放宽为 90 秒，仍取三轮中位数并保留 256 MiB 内存约束。另有 Rust 单元测试断言
调度器的活跃操作数不超过配置上限且维持输入顺序。两层门禁共同捕获无界 task 创建与聚合
内存回退，同时无需在仓库中保存大型字体语料。

构建输出使用独立的四路原子写入上限，`threads` 控制逐文件处理。每个父目录只准备
一次。写入仍在原子替换前同步临时文件，并检查符号链接与重复目标，包括经由父目录
符号链接指向同一文件的情况。某次写入失败时，会等待当前批次完成后再返回错误，
使临时文件清理能够完成。
文件名可能存在大小写或 Unicode 别名，或与临时文件命名相似时，当前批次回退为
按顺序写入，保留覆盖顺序，并避免与其他输出的临时文件冲突。

规模 stage 在计时结束后验证每个文件名与字节，并记录文件名和内容的 SHA-256 摘要，
用于跨轮次比较。Node 缓存 stage 以八路并发请求执行 512 次写入，包含默认 256 条
缓存上限触发的淘汰。同一进程对相同缓存目录的请求先排队，再获取跨进程文件锁，
避免本地竞争产生的 25 ms 重试等待；不同目录仍可同时推进。

字体批处理 stage 另行覆盖真实 subset 与 WOFF2 工作：八个输入使用固定版本、
17,773,132 字节的 Noto Sans SC variable font，分别设置一个与四个 CLI worker slot。
每个输入选取相同的 Latin、中文及标点文本，输出一个 WOFF2 文件。各副本具有独立路径
和 cache key；临时硬链接避免在磁盘上重复存储 fixture。

每一轮从独立的应用缓存开始。冷缓存测量包含 subset、压缩、缓存填充与输出写入。
热缓存轮次先执行一次不计时构建并验证结果，随后删除输出目录；新的计时 CLI 进程必须
从缓存恢复全部输出，且不能重写缓存 index。预热内存不计入被测子进程 RSS。冷热状态
指应用缓存，并不表示操作系统页缓存已清空。Fixture 准备和输出验证不计入耗时。

输出验证检查文件名集合、完整的 WOFF2 header 与长度、CLI inspection 成功、请求文本
的完整字符覆盖、非空且减少的 glyph 集合，以及保留 `fvar`/`gvar` 表。文件名与内容的
SHA-256 摘要必须在
三轮间一致，热缓存恢复结果也必须与预热结果相同，避免将缺失、损坏或旧输出误判为提速。

新增字体批处理预算采用保守初始上限，尚未在固定 Ubuntu runner 上校准。macOS 本地
运行使用子进程 RSS 采样；Linux 门禁采样内核记录的进程内存峰值。本地结果用于验证
场景，不能直接作为 Ubuntu 性能基线。

无论预算是否失败，CI 都会上传 `benchmarks/production-current.json`。报告会为每个
stage 保存三轮耗时与内存、聚合指标、预算、输出字节数、状态和具体 violation。字体
批处理还记录输入数、worker 数、缓存状态和输出摘要。绝对预算只在固定 runner 上作为
发布门禁；宿主环境不同的本地报告主要用于诊断。

已提交的 [`benchmarks/baseline.json`](../../benchmarks/baseline.json) 会记录机器指纹、
fixture checksum、三轮独立均值、中位数指标和性能判定。只能通过以下命令重录：

```sh
pnpm run bench:baseline
```

提交新基线前必须审查完整 diff。性能变慢时，需要再进行三轮同环境复测，并修复问题或
记录有意保留的正确性取舍。

## 缓存与输出优化（2026-10-05）

[优化前后报告](../../benchmarks/cache-output-2026-10-05.json) 记录了 Apple M1 Pro、
macOS arm64、Node.js 24.21.0、当前 shell 的 Homebrew Rust 1.99.0 环境下的三轮
release profile 测量。完整仓库检查使用固定的 Rust 1.98.0，报告保留基准实际使用的
编译器以便复现。两份构建包含相同的
既有工作区修改。耗时取中位数，内存取三轮观察到的最大进程 RSS。

| 场景                            |   优化前 |   优化后 | 加速比 |  峰值 RSS 前 → 后 |
| ------------------------------- | -------: | -------: | -----: | ----------------: |
| Node 缓存，512 次写入／8 路请求 | 12.464 s |  1.099 s | 11.34× | 93.89 → 91.38 MiB |
| CLI，10,000 个单字节输入        | 54.732 s | 31.769 s |  1.72× | 12.84 → 15.94 MiB |

CLI 规模场景与真实字体批处理的所有产物摘要在优化前后保持一致，包括一个和四个
处理 worker 的冷热缓存场景。缓存 stage 的时间戳会随运行变化，因此比较索引字节数；
文件锁与缓存正确性由独立回归测试覆盖。这些本地结果不调整固定 Ubuntu 预算。
重建 release CLI、binding 和包后，可将报告中的 stage 名传给
`production-performance-worker.mjs` 复测。

## 当前候选版基线

已提交的 1.0.2-rc.1 release profile 基线中，代表性 fontmin-rs pipeline 的平均耗时
比例为经典 Fontmin 的 0.1829，约快 5.47 倍。`subsetTtf text` 为 1.5135 ms，历史
beta.3 快照为 1.0912 ms；另一份三轮报告复现了该变化。聚焦测量表明，成本来自新的
subset 引擎和修正后的 `keepLayout: "conservative"` 布局重映射；历史路径会静默丢弃
layout tables。

Subset、WOFF、WOFF2、SVG 和 modern-web pipeline 的绝对耗时仍保留在报告中用于诊断。
由于不同任务间的 CPU 配额可能变化，hosted runner 的绝对耗时只作为证据，不设为硬门禁。

运行 `pnpm run bench:profile` 可对代表性 pipeline 进行粗粒度 CPU profile。它会执行
2,500 次 release binding，并在 `benchmarks/` 下写入被忽略的 `.cpuprofile`。
beta.3 profile 表明 glyph subsetting 是最大的具名耗时块，JavaScript pipeline
调度并非主要热点。1.0.2-rc.1 基线明确接受保留并重映射受支持 layout 数据的测量成本。
