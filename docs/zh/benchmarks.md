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

## Production 耗时与内存预算

`pnpm run bench:production` 会先运行 conformance，再让每个 production stage 在独立
Node.js 进程中执行。每个 stage 采集三轮数据：耗时取中位数，避免一次调度中断被误判为
回退；内存则取最大的进程 `maxRSS`。Stage 隔离使失败能直接指出对应 runtime、操作和
fixture。

已提交的
[`benchmarks/production-budgets.json`](../../benchmarks/production-budgets.json)
定义 Ubuntu 24.04 与 Node.js 24 门禁：

| Stage 类别           | 最大耗时中位数 | 最大 peak RSS |
| -------------------- | -------------: | ------------: |
| Native inspect       |         500 ms |       128 MiB |
| WASM 初始化          |         250 ms |       128 MiB |
| WASM inspect         |         250 ms |       160 MiB |
| Native 混合 delivery |         500 ms |       192 MiB |
| WASM 混合 delivery   |       1,000 ms |       256 MiB |

无论预算是否失败，CI 都会上传 `benchmarks/production-current.json`。报告会为每个
stage 保存三轮耗时与内存、聚合指标、预算、输出字节数、状态和具体 violation。绝对
预算只在固定 runner 上作为发布门禁；宿主环境不同的本地报告主要用于诊断。

已提交的 [`benchmarks/baseline.json`](../../benchmarks/baseline.json) 会记录机器指纹、
fixture checksum、三轮独立均值、中位数指标和性能判定。只能通过以下命令重录：

```sh
pnpm run bench:baseline
```

提交新基线前必须审查完整 diff。性能变慢时，需要再进行三轮同环境复测，并修复问题或
记录有意保留的正确性取舍。

## 当前结果

Release profile 基线中，代表性 fontmin-rs pipeline 的平均耗时比例为经典 Fontmin 的
0.1485，约快 6.73 倍。此前 debug profile 快照造成的落后结论并非产品回退；改用
release profile 门禁后，该测量错误已经消除。

Subset、WOFF、WOFF2、SVG 和 modern-web pipeline 的绝对耗时仍保留在报告中用于诊断。
由于不同任务间的 CPU 配额可能变化，hosted runner 的绝对耗时只作为证据，不设为硬门禁。

运行 `pnpm run bench:profile` 可对代表性 pipeline 进行粗粒度 CPU profile。它会执行
2,500 次 release binding，并在 `benchmarks/` 下写入被忽略的 `.cpuprofile`。
beta.3 profile 表明 glyph subsetting 是最大的具名耗时块，JavaScript pipeline
调度并非主要热点。由于配对门禁已经明显优于 parity，beta.3 不需要记录任何以正确性换取
性能的取舍。
