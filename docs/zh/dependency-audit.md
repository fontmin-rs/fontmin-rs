# 依赖与制品体积审计

发布策略要求每一组重复 Rust 依赖和本地 crate override 都有明确决策，并为三种可执行交付面
设置体积预算。机器可读的唯一事实来源是
[`audits/release-policy.json`](../../audits/release-policy.json)。

## 重复依赖决策

2026-08-30 的依赖升级消除了 Brotli 与 thiserror 的重复主版本，目前保留四组已审计重复依赖：

| 依赖            | 版本            | 决策 | 替换条件                                                    |
| --------------- | --------------- | ---- | ----------------------------------------------------------- |
| `hashbrown`     | 0.15.5 / 0.17.1 | 保留 | 等待 `wasmi`/`string-interner` 与 `indexmap` 链统一。       |
| `miniz_oxide`   | 0.8.9 / 0.9.1   | 保留 | 等待 `backtrace` 与 `flate2` 链统一。                       |
| `syn`           | 2.0.119 / 3.0.4 | 保留 | 等待其余过程宏依赖迁移至 syn 3。                            |
| `unicode-width` | 0.1.14 / 0.2.2  | 保留 | 等待 `miette`/`textwrap` 在不改变诊断输出的前提下统一版本。 |

所有决策均由 fontmin-rs maintainers 负责。新增重复项、已记录版本变化，或重复项消失但保留
决策未删除，都会使依赖门禁失败。

## Vendored patch 决策

| Crate                  | 上游                                                                  | 决策与退出条件                                                                                    |
| ---------------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `allsorts` 0.17.0      | [yeslogic/allsorts](https://github.com/yeslogic/allsorts)             | 保留 CFF INDEX 与 `endchar` 修正；上游版本具备等价行为且永久回归语料通过后移除。                  |
| `oxifont-core` 0.2.2   | [cool-japan/oxifont](https://github.com/cool-japan/oxifont)           | 保留仅修改 MSRV 元数据的补丁，直到上游声明支持 Rust 1.88，或本项目以 SemVer minor 版本提高 MSRV。 |
| `oxifont-subset` 0.2.2 | [cool-japan/oxifont](https://github.com/cool-japan/oxifont)           | 保留安全选择、CFF/CFF2、CID、COLR v1 与 cmap 修正，直到等价上游版本通过回归语料和依赖审计。       |
| `safer-bytes` 0.2.0    | [danieleades/safer-bytes](https://github.com/danieleades/safer-bytes) | 保留 stable-Rust compatibility copy，直到选定的 WOFF2 decoder 不再依赖它。                        |
| `woff2-patched` 0.4.0  | [zimond/woff2-rs](https://github.com/zimond/woff2-rs)                 | 保留显式坐标 wrapping；上游版本或自有 decoder 通过全部 WOFF2 回归后替换。                         |

每个 override 的源码旁都有 patch notes。审计会同时验证 root Cargo patch、说明、负责人、
上游地址、决策与移除条件。两个 oxifont 副本都以发布版本 0.2.2 为基础。
`oxifont-core` 只把 manifest 声明的 Rust 版本从 1.89 降到 1.88；
`oxifont-subset` 移除了未使用的生产依赖 `oxifont-parser` 与无人维护的
`ttf-parser`，并携带上表所列的源码级子集安全修正。CI 会使用 Rust 1.98 编译完整
workspace。本次 MSRV 提升已满足 `oxifont-core` 元数据补丁原先记录的退出条件，移除该
override 需要单独进行依赖图审计。oxifont-subset 的上游准备与提交映射记录在
`audits/oxifont-subset-upstream-2026-08-31.md`。

## Release 制品体积预算

Release build 启用 thin LTO、单 codegen unit 与符号裁剪。预算为受支持 CI 平台保留余量：

| 制品                |  预算 |
| ------------------- | ----: |
| Rust CLI            | 8 MiB |
| Native Node binding | 8 MiB |
| Browser WASM binary | 5 MiB |

在当前功能集下，macOS arm64 本地实测为：CLI 7,139,040 bytes、native binding
5,678,928 bytes、WASM 4,731,426 bytes。WASM 预算为浏览器端变量字体裁剪以及与源文件
绑定的子集计划功能预留了余量。CI 会把平台实测写入
`audits/artifact-current.json`，并与性能报告一起上传。

只检查依赖决策：

```shell
pnpm run audit:dependencies
```

构建并测量全部 release 交付面：

```shell
pnpm run audit:artifacts
```

体积超限时仍会先持久化完整报告，便于直接定位对应交付面和实际字节数。
