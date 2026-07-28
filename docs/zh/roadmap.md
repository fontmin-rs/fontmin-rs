# 1.0 路线图与后续计划

fontmin-rs `1.0.0` 是覆盖 CLI、Node.js 包、浏览器 WASM 包、native binding 和
8 个平台包的稳定版本。以下里程碑保留已完成的计划及退出证据。

路线图采用退出条件，不承诺日历日期。只有相关检查能在 `main` 重复通过、打包后的
消费者 smoke test 覆盖受影响公开路径，并且发布流程仍可复现时，里程碑才算完成。

## 稳定基线

- 发布门禁会核对 11 个 npm 包、Cargo metadata、运行时内嵌版本、Changelog 和
  release tag 是否使用同一版本。
- CI 覆盖格式化、Rust/TypeScript 零警告 lint、Node.js 22.18/24/26、WASM、浏览器
  加载、文档、native package smoke test、发布准备检查和 benchmark。
- 发布前会拒绝 high/critical 依赖安全问题，要求 Rust 行覆盖率不低于 80%，检查
  npm tarball 内容，并运行消费者 smoke test。
- Native 与 WASM 共享语义一致性 corpus，覆盖内置 transforms、presets、输出
  metadata 和 malformed diagnostics。
- 有时间上限的 AddressSanitizer cargo-fuzz 会在相关改动和每周任务中运行；
  最小化后的 crash 会成为永久 malformed fixture。
- Rust `1.88.0` 是独立测试的 MSRV；开发与发布自动化使用仓库声明的固定 toolchain。

## 0.1.1——契约修正

首个补丁版本修复 `0.1.0` 发布后立即发现的漂移，不扩张公开 API。

- 将 README、安装指南、导航标签和机器可读清单统一为稳定版状态。
- 让 npm 包内 CLI 接受所有已冻结的 Rust CLI 参数，并用同一份契约验证两个
  executable。
- 将 Rust、Node.js 与浏览器 pipeline 的 SVG icon-font 默认 stem 统一为
  `iconfont`。
- 增加语义检查，防止稳定 package version 再发布预发布安装指引。

退出条件：完整发布门禁通过，npm tarball CLI 与 Rust CLI 均符合公开清单，并将
`0.1.1` 发布到 `latest`。

## 0.2——收敛流水线边界

`0.2` 发布线在保留稳定公开入口的同时，减少重复策略。

- 将 npm executable 变为共享命令解析与 pipeline 行为之上的薄适配层，不再维护
  第二套独立 CLI。
- 在 `fontmin_config` 中把内置 plugin 配置规范化为类型化领域值，移除
  `fontmin_pipeline` 中重复的 JSON option decoding。
- 将已知的 `AssetMeta.custom` key 替换为类型化 metadata，同时保留第三方 plugin
  使用的扩展 map。
- 按 pipeline execution、transform rules、filesystem/cache ownership 拆分 Node
  optimizer，同时保留现有 `optimize()` facade。
- 按公开 command 或 API seam 拆分过大的 CLI 与 Node integration tests。
- 明确 Rust workspace crates 是仅内部使用还是独立发布，并让 Cargo manifests
  强制执行该决策。
- 将 `0.1.0` 前的设计提案明确标记为历史资料；当前架构和契约文档是事实来源。

退出条件：所有公开入口保持契约，重复的 CLI 与配置规则只有一处事实来源，完整
conformance 和 package gates 在没有兼容性例外的情况下通过。

## 0.3——真实场景韧性与性能

`0.3` 发布线通过生产规模输入积累证据，并降低剩余运行风险。

- 扩展大体积 CJK、variable font、color font、malformed tables 和混合 delivery
  slices 的一致性 fixtures。
- 为 native 与 WASM 增加有上限的内存和耗时预算，回退报告能够定位具体阶段。
- 从真实故障扩充 fuzz corpus，并覆盖 parser、converter、配置加载和输出命名。
- 审计重复压缩/错误处理依赖、vendored patches、binary size 与上游替代路径。
- 验证缓存并发、取消和多进程构建中断后的清理行为。

退出条件：代表性大字体保持在文档化性能预算内，每条支持的格式路径都有回归 fixture，
且已知 vendored/dependency 风险都有负责人和替代决策。

## 1.0——独立验证的公开契约

`1.0.0` 不是对 `0.1` 契约的简单改名。它会在真实项目证据形成最终边界后，启动独立
RC 周期。

- 收集 CLI、Node.js 和浏览器消费者的兼容性报告。
- 为每项有意的契约变化发布迁移说明。
- 完成已公告的弃用周期，仅移除符合弃用策略的 API。
- 重新确认 runtime、native targets、浏览器能力、Rust MSRV、diagnostics 与生成
  文件命名规则。
- 至少通过一个 RC 周期，且不存在未解决的 P0/P1 正确性、安全、性能或打包问题。

`1.0.0-rc.1` 已完成该周期：全部独立消费者项目和精确标签的边界流程均通过，就绪审计
没有记录未解决的 P0/P1 问题。稳定版提升只接受 `1.0.0` 所需的版本常量变化，经过
审阅的运行时行为保持不变。

退出条件：独立版本的 RC 契约通过完整发布门禁和真实项目验证，同一份经过审阅的运行时
实现无需发布时修补即可提升为 `1.0.0`。

## 1.0 之后

- `1.0.x`：仅做兼容性修复，保留所有最小化回归样本，并为每个稳定版本发布 registry
  兼容性证据。
- `1.1`：只在真实消费者需求、一致性 fixtures 和 native/WASM 性能预算共同明确行为
  后增加 API。
- 维护工作：存在可验证的上游替代路径时，继续替换 vendored 或无人维护的依赖；使用同一
  份清单检查契约、文档、包和 CI matrix。
- 只有完成弃用策略与新一轮独立兼容性审阅后，才规划破坏性版本。
