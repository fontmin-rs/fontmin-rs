# 依赖与制品体积审计

发布策略要求每一组重复 Rust 依赖和本地 crate override 都有明确决策，并为三种可执行交付面
设置体积预算。机器可读的唯一事实来源是
[`audits/release-policy.json`](../../audits/release-policy.json)。

## 重复依赖决策

2026-07-28 的审计记录了五组重复依赖：

| 依赖                  | 版本            | 决策 | 替换条件                                                    |
| --------------------- | --------------- | ---- | ----------------------------------------------------------- |
| `brotli`              | 7.0.0 / 8.0.4   | 保留 | 移除 patched WOFF2 decoder 链时一并移除 v7。                |
| `brotli-decompressor` | 4.0.3 / 5.0.3   | 保留 | 随 Brotli v7 一并移除 v4。                                  |
| `thiserror`           | 1.0.69 / 2.0.18 | 保留 | 移除两个 WOFF2 compatibility crate 时移除 v1。              |
| `thiserror-impl`      | 1.0.69 / 2.0.18 | 保留 | 跟随已审计的 `thiserror` 版本。                             |
| `unicode-width`       | 0.1.14 / 0.2.2  | 保留 | 等待 `miette`/`textwrap` 在不改变诊断输出的前提下统一版本。 |

所有决策均由 fontmin-rs maintainers 负责。新增重复项、已记录版本变化，或重复项消失但保留
决策未删除，都会使依赖门禁失败。

## Vendored patch 决策

| Crate                  | 上游                                                                  | 决策与退出条件                                                                                      |
| ---------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `allsorts` 0.17.0      | [yeslogic/allsorts](https://github.com/yeslogic/allsorts)             | 保留 CFF INDEX 与 `endchar` 修正；上游版本具备等价行为且永久回归语料通过后移除。                    |
| `oxifont-core` 0.2.2   | [cool-japan/oxifont](https://github.com/cool-japan/oxifont)           | 保留仅修改 MSRV 元数据的补丁，直到上游声明支持 Rust 1.88，或本项目以 SemVer minor 版本提高 MSRV。   |
| `oxifont-subset` 0.2.2 | [cool-japan/oxifont](https://github.com/cool-japan/oxifont)           | 保留 manifest 补丁，直到上游支持 Rust 1.88 并移除未使用的生产 parser 依赖，或满足已审计的退出条件。 |
| `safer-bytes` 0.2.0    | [danieleades/safer-bytes](https://github.com/danieleades/safer-bytes) | 保留 stable-Rust compatibility copy，直到选定的 WOFF2 decoder 不再依赖它。                          |
| `woff2-patched` 0.4.0  | [zimond/woff2-rs](https://github.com/zimond/woff2-rs)                 | 保留显式坐标 wrapping；上游版本或自有 decoder 通过全部 WOFF2 回归后替换。                           |

每个 override 的源码旁都有 patch notes。审计会同时验证 root Cargo patch、说明、负责人、
上游地址、决策与移除条件。两个 oxifont 副本保留发布的 0.2.2 Rust 源码。
`oxifont-core` 只把 manifest 声明的 Rust 版本从 1.89 降到 1.88；
`oxifont-subset` 还移除了未使用的生产依赖 `oxifont-parser` 与无人维护的
`ttf-parser`，其 `src/` 目录没有引用二者。CI 会使用 Rust 1.88 编译完整 workspace。

## Release 制品体积预算

Release build 启用 thin LTO、单 codegen unit 与符号裁剪。预算为受支持 CI 平台保留余量：

| 制品                |  预算 |
| ------------------- | ----: |
| Rust CLI            | 8 MiB |
| Native Node binding | 8 MiB |
| Browser WASM binary | 4 MiB |

启用 release profile 后，macOS arm64 本地实测为：CLI 4,425,520 bytes、native binding
3,279,968 bytes、WASM 2,964,395 bytes。CI 会把平台实测写入
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
