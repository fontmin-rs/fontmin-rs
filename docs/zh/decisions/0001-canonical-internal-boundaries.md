# ADR 0001：规范化内部边界

- 状态：已采纳
- 日期：2026-07-29

## 背景

相同行为曾分散在多个公开入口的 adapter 中：Node 与 WASM 各自维护相近的优化策略，
多个格式 crate 分别写入 sfnt directory，native 发布脚本重复平台映射，而 Rust CLI
的 build command 同时承担编排、缓存持久化、锁和输出安全。

这类重复让局部修改变得危险。新增 native target、输出安全规则、diagnostic 行为或
sfnt invariant 时，容易只更新一条路径，其他路径则在没有明显错误的情况下发生漂移。

## 决策

每个跨模块 invariant 只保留一个 canonical owner，公开 facade 保持轻量：

| Invariant                                       | Canonical owner                             |
| ----------------------------------------------- | ------------------------------------------- |
| Node workspace 读取、路径展开、安全清理与写入   | `packages/fontmin/src/workspace-io.ts`      |
| Node/WASM optimizer policy 与 diagnostic 规范化 | `packages/fontmin/src/runtime-neutral/`     |
| Rust CLI 缓存持久化与锁                         | `apps/fontmin/src/commands/build/cache.rs`  |
| Rust CLI 输出 containment 与受保护清理          | `apps/fontmin/src/commands/build/output.rs` |
| sfnt directory 校验与序列化                     | `crates/fontmin_ttf/src/sfnt.rs`            |
| native target 到 package 的布局                 | `scripts/native-release-layout.mjs`         |
| CLI integration test 的进程与临时 workspace     | `apps/fontmin/tests/cli/support.rs`         |

SVG icon-font 内部按职责拆为 markup 提取、path geometry 和 TTF table 构造；公开函数仍
保留在 `crates/fontmin_svg/src/icon.rs`。

N-API 的 `targets` 数组是 native target inventory 的权威来源。平台 package 名称、
目录、artifact 名称、runtime 选择与 metadata 校验均从它推导。Workflow matrix
继续使用便于 review 的 YAML，并由测试阻止它与 inventory 漂移。

## 影响

- 跨 runtime 的策略修改只有一个实现，并且必须同时通过 Node 与 WASM 测试。
- 格式 crate 不再自定义 sfnt directory 机制，只向 canonical writer 提供 table data。
- 缓存与输出安全可以独立于 CLI build 编排演进。
- 新增 native target 时需要补充 N-API target、平台 manifest 与 workflow runner；
  校验会报告所有不一致入口。
- 内部模块可以保持私有且带有明确约束；公开 API 兼容性仍由 `contracts/` 与弃用策略
  管理。

代价是内部模块与接口数量增加。各目录的 `CONTEXT.md` 记录 ownership，使新增行为
优先深化已有模块，而不是重新建立平行实现。

## 未采用的方案

- 依赖约定手工同步重复实现。已有的独立清单与 serializer 已经增加了审计成本。
- 创建包含所有共享代码的通用 utility crate/package。不同关注点的依赖和变化速度
  不同，宽泛的 utility 会削弱 locality。
- 从代码生成 CI workflow YAML。仓库保留可直接 review 的 workflow 配置，以漂移
  测试替代生成步骤。
