# Oxc 与 Rolldown 仓库目录结构调研

## 调研范围

本文只记录上游仓库的结构事实与可观察到的组织模式，不对 `fontmin-rs` 应采取的调整作结论。

- 调研日期：2026-07-11。
- Oxc 基准：`oxc-project/oxc` 的 `main` 提交 [`8da040290cc021376d64e5621e1da4fe214bd14f`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f)。
- Rolldown 基准：`rolldown/rolldown` 的 `main` 提交 [`b9823050bc658ef65105148ea0504d4fbda7fa4c`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c)。
- 资料仅取自上述官方 GitHub 仓库中的源码、清单、开发文档和 GitHub Actions 工作流。

本仓库现有 `docs/` 主要包含用户文档，以及按日期归档的 `docs/superpowers/plans/` 和 `docs/superpowers/specs/`；VitePress 配置排除 `superpowers/**/*.md`。此前没有独立的研究记录目录，因此本文放在新建的 `docs/superpowers/research/` 中，并采用日期前缀命名，以免进入用户文档站。

## 顶层结构对照

| 关注点 | Oxc | Rolldown |
| --- | --- | --- |
| Rust workspace | `apps/*`、`crates/*`、`napi/*`、`tasks/*` 都可进入 workspace，同时显式排除 `apps/shared`、`tasks/lint_rules`、`tasks/e2e`。[`Cargo.toml`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/Cargo.toml) | workspace 只覆盖 `crates/*` 和 `tasks/*`。[`Cargo.toml`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/Cargo.toml) |
| JavaScript workspace | 覆盖 `apps/*`、`napi/*`、`wasm/*`、`npm/*`，并选择性包含部分 `tasks/*`。[`pnpm-workspace.yaml`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/pnpm-workspace.yaml) | 覆盖 `packages/*`、`docs`、`examples/*`、`scripts`，还把少量需要独立依赖图的测试夹具目录列为 workspace package。[`pnpm-workspace.yaml`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/pnpm-workspace.yaml) |
| 产品与库 | `apps/` 放最终 CLI 应用，`crates/` 放可组合的 Rust 组件，`napi/` 放 Node.js 绑定，`npm/` 放 npm 交付包。[顶层目录](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f)、[`ARCHITECTURE.md`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/ARCHITECTURE.md) | Rust 核心和 Node 绑定分别位于 `crates/rolldown`、`crates/rolldown_binding`；主 npm 包、浏览器包和测试/调试包统一位于 `packages/`，没有顶层 `apps/`、`napi/` 或 `npm/`。[结构说明](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs/development-guide/repo-structure.md)、[`packages/`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/packages) |
| 文档 | 仓库根目录保留 `README.md`、`ARCHITECTURE.md`、`CONTRIBUTING.md`、`MAINTENANCE.md`；当前提交没有顶层 `docs/`，`justfile` 中的网站联调命令指向独立的 `oxc-project/website` 仓库。[顶层目录](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f)、[`justfile`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/justfile)、[官方 website 仓库](https://github.com/oxc-project/website) | `docs/` 是站点及公开开发文档，`internal-docs/` 保存实现设计主题；根目录另有贡献与维护文档。[`docs/`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs)、[`internal-docs/`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/internal-docs) |
| 示例 | 没有顶层 `examples/`；Rust 示例放在所属 crate 的 `examples/`，Node parser 示例放在 `napi/parser/example.js`。[顶层目录](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f)、[`crates/oxc_parser/examples`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/crates/oxc_parser/examples)、[`napi/parser/example.js`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/napi/parser/example.js) | 顶层 `examples/` 按使用场景拆成独立小项目，并被 pnpm workspace 纳入。[`examples/`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/examples)、[`pnpm-workspace.yaml`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/pnpm-workspace.yaml) |
| Benchmark | 集中在 Rust 工具 crate `tasks/benchmark`，按 parser、transformer、linter、minifier 等组件拆 bench；N-API parser 另有邻近包的 JS benchmark。[`tasks/benchmark`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/tasks/benchmark)、[`napi/parser/bench.bench.js`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/napi/parser/bench.bench.js) | Rust benchmark 位于 `crates/bench`，Node.js benchmark 位于 `packages/bench`，输入准备脚本位于 `scripts/misc/setup-benchmark-input`。[`crates/bench`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/crates/bench)、[`packages/bench`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/packages/bench)、[benchmark 开发文档](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs/development-guide/benchmarking.md) |
| Tasks 与 scripts | `tasks/` 同时容纳代码生成、覆盖率/一致性测试、benchmark、体积/内存跟踪和网站数据生成等 Rust 或混合语言工具；自动化脚本还分布在 `.github/scripts/` 与具体 app/N-API 包内。[`tasks/`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/tasks)、[`.github/scripts/`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/.github/scripts) | `tasks/` 放 Rust 生成器、目录命名 lint、内存分配跟踪等内部二进制；`scripts/` 是独立 pnpm workspace，放 lint、发布检查、测试生成和 benchmark 输入准备等 JS/TS 自动化。[`tasks/`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/tasks)、[`scripts/`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/scripts) |

## Rust、Node 绑定与发布包的边界

### Oxc

Oxc 的 `crates/` 是细粒度编译器组件集合；`apps/oxlint` 与 `apps/oxfmt` 是应用层，二者目录内同时包含 Rust CLI/N-API 入口、JS 包装代码、构建脚本与测试。架构文档把 `apps/oxlint` 明确称为 application layer，并把 `napi/*` 描述为 Node.js integration layer。[`ARCHITECTURE.md`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/ARCHITECTURE.md)、[`apps/`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/apps)

`napi/minify`、`napi/parser`、`napi/transform` 每个目录本身都是 Rust crate 与 npm package 的组合单元，包含 `Cargo.toml`、`package.json`、Rust `src/`、JS/TS 入口、构建脚本和测试；`napi/playground` 也采用相同交付形态。[`napi/`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/napi)

`npm/` 不等同于 N-API 二进制清单集合：它还容纳 `oxc-types`、`runtime`、Oxlint 插件包，以及 `oxlint`、`oxfmt` 的 npm 发布包装。[`npm/`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/npm)

Oxc 没有在源码树中长期维护一组平台专属 npm 目录；release workflow 汇总构建产物后调用 `napi create-npm-dirs` 与 `napi artifacts` 生成平台包并发布。[`release_apps.yml`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows/release_apps.yml)、[`reusable_release_napi.yml`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows/reusable_release_napi.yml)

### Rolldown

Rolldown 将 Rust N-API 胶水保留为 `crates/rolldown_binding`，但 JS API、CLI、构建脚本、绑定加载代码与 Node 测试都放进 `packages/rolldown`。因此其边界是“Rust crate 集合”与“Node package 集合”，而不是独立的顶层 `napi/`/`npm/` 两阶段目录。[`crates/rolldown_binding`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/crates/rolldown_binding)、[`packages/rolldown`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/packages/rolldown)

`packages/` 还包含 `browser`、`debug`、`test-dev-server`、`rollup-tests`、`vite-tests` 和 benchmark 等具备独立依赖或执行职责的包。[`packages/`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/packages)

Rolldown 的平台专属 npm 目录也不常驻当前源码树；`publish-to-npm.yml` 在下载多平台 artifacts 后，于 `packages/rolldown` 中运行 `napi create-npm-dirs` 和包内的 `artifacts` 脚本生成 `npm/` 与发布文件。[`publish-to-npm.yml`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/.github/workflows/publish-to-npm.yml)、[`packages/rolldown/package.json`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/packages/rolldown/package.json)

## 测试与测试资产

### Oxc

- Rust crate 的单元/集成测试与源码共置在各 crate 中；应用与 N-API 的 JS/TS 测试分别位于所属 `apps/*/test`、`napi/*/test`，formatter/linter 的 conformance 资产也位于所属 app 下，而不是集中到根级 `fixtures/`。[`apps/oxfmt`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/apps/oxfmt)、[`apps/oxlint`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/apps/oxlint)、[`napi/`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/napi)
- 大型规范/兼容性套件由 `tasks/coverage`、`tasks/prettier_conformance`、`tasks/transform_conformance` 等专用 task 驱动；覆盖率文档列出 Test262、Babel、TypeScript 等上游套件。[`tasks/coverage/README.md`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/tasks/coverage/README.md)、[`tasks/`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/tasks)
- 这些大体积上游资产不是普通 Git submodule 清单：`.github/scripts/clone-parallel.mjs` 固定每个上游的提交 SHA，将它们浅克隆到对应 task 目录；`update_submodules.yml` 负责更新固定 SHA 与相关 fixture/snapshot。[克隆脚本](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/.github/scripts/clone-parallel.mjs)、[更新工作流](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows/update_submodules.yml)
- 回归输出与基准状态通常保存在所属工具目录：例如 `tasks/coverage/snapshots`、`tasks/transform_conformance/snapshots`、`tasks/track_memory_allocations/*.snap` 与 app 内 snapshots。[`tasks/coverage`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/tasks/coverage)、[`tasks/track_memory_allocations`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/tasks/track_memory_allocations)

### Rolldown

- 官方测试指南将测试分成 Rust 与 Node.js 两组，并说明由于 bundler 的性质，主要使用端到端、数据驱动的集成测试。Rust 测试位于 `crates/rolldown/tests`，以含 `_config.json` 的目录作为用例；Node API 测试位于 `packages/rolldown/tests`，数据驱动 fixture 使用 `_config.ts`。[测试指南](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs/development-guide/testing.md)
- `crates/rolldown/tests` 同时承载原生 Rolldown 用例、从 esbuild 派生的兼容测试及其 `artifacts.snap`；管理和比对 esbuild 用例的工具放在 `scripts/src/esbuild-tests`。[Rust 测试目录](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/crates/rolldown/tests)、[esbuild 脚本](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/scripts/src/esbuild-tests)
- Rollup 与 Test262 作为根级 Git submodule；`packages/rollup-tests` 是用 Rolldown 运行 Rollup 官方测试的适配器，维护通过/失败/忽略状态。[`.gitmodules`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/.gitmodules)、[`packages/rollup-tests/README.md`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/packages/rollup-tests/README.md)
- Dev engine 的浏览器/Node 端到端 harness 独立为 `packages/test-dev-server`，其测试 playground/fixture 被 pnpm workspace 显式纳入；对应设计说明在 `internal-docs/dev-server-test-harness`。[测试指南](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs/development-guide/testing.md)、[`internal-docs/dev-server-test-harness`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/internal-docs/dev-server-test-harness)

## CI、benchmark 与 release

### Oxc

- CI 工作流按职责拆分，包括主 CI、PR、benchmark、bloat、coverage、deny、ecosystem CI、Miri、安全检查和规则生成等。[`.github/workflows`](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows)
- benchmark 工作流只在相关 Rust 源码、`tasks/benchmark`、`tasks/common` 或工具链文件变化时触发，并根据受影响组件生成矩阵，再由 CodSpeed 分片执行。[`benchmark.yml`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows/benchmark.yml)、[矩阵脚本](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/.github/scripts/generate-benchmark-matrix.js)
- 发布流程按产物家族拆分：apps、Rust crates、各 N-API package、runtime 和 types 各有 prepare/release 工作流；N-API 三个包复用 `reusable_release_napi.yml`。[发布工作流目录](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows)、[`reusable_release_napi.yml`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows/reusable_release_napi.yml)
- Oxlint 与 Oxfmt 作为同一 apps 发布流程一起准备和构建，但各自版本可不同；Rust crates 使用另一条发布流程。[`MAINTENANCE.md`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/MAINTENANCE.md)、[`prepare_release_apps.yml`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows/prepare_release_apps.yml)

### Rolldown

- CI 除主流程外，把 Cargo、Node、dev-server、native build、WASI 与 release build 抽成 reusable workflows；另有站点部署、链接检查、benchmark、依赖测试资产更新和生态触发流程。[`.github/workflows`](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/.github/workflows)
- benchmark 明确分为 `benchmark-rust.yml` 与 `benchmark-node.yml`：Rust 使用 `crates/bench`/CodSpeed，Node 使用 `packages/bench`，结果可写入独立的官方 benchmark 存储仓库。[`benchmark-rust.yml`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/.github/workflows/benchmark-rust.yml)、[`benchmark-node.yml`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/.github/workflows/benchmark-node.yml)
- 正式 npm 发布由 `prepare-release.yml` 先创建版本 PR，合并后由 `publish-to-npm.yml` 构建多平台产物、发布 npm 并创建 GitHub Release；preview 使用独立 `publish-to-pkg.pr.new.yml`，Rust crates 又由 `release-crates.yml` 单独发布。[`MAINTENANCE.md`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/MAINTENANCE.md)、[`publish-to-npm.yml`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/.github/workflows/publish-to-npm.yml)、[`release-crates.yml`](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/.github/workflows/release-crates.yml)

## 可观察到的组织原则

以下是从上述目录与官方说明中归纳的事实模式，不是对本仓库的调整建议：

1. **顶层目录反映交付边界，而非固定模板。** Oxc 为 CLI、Rust 组件、N-API 绑定和 npm 包分别使用 `apps/`、`crates/`、`napi/`、`npm/`；Rolldown 则把 N-API 胶水归入 Rust crates，把所有 Node 交付面归入 `packages/`。[Oxc 顶层](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f)、[Rolldown 结构说明](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs/development-guide/repo-structure.md)
2. **Cargo 与 pnpm workspace 不要求镜像。** 两个仓库都按各自构建系统需要显式选择成员；Rolldown 甚至把部分 fixture 目录注册成 pnpm package，而不因此改变 Rust 目录。[Oxc workspaces](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/pnpm-workspace.yaml)、[Rolldown workspaces](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/pnpm-workspace.yaml)
3. **没有共同采用根级 `fixtures/`。** 小型 fixture、snapshot 和测试通常与所属 crate/app/package 共置；只有跨组件、体量大或需独立更新的规范/兼容套件才提升为 `tasks/`、专用 package 或根级外部资产。[Oxc tasks](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/tasks)、[Rolldown 测试指南](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs/development-guide/testing.md)
4. **`tasks/` 对应“仓库开发工具”，但语言边界不同。** Oxc 把大量 Rust/混合语言 benchmark、conformance、codegen 工具统一放在 `tasks/`；Rolldown 用 `tasks/` 放 Rust 内部工具，用单独的 `scripts/` workspace 放 JS/TS 自动化。[Oxc tasks](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/tasks)、[Rolldown tasks/scripts](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs/development-guide/repo-structure.md)
5. **benchmark 与 release 的目录/工作流随运行时和产物家族拆分。** Oxc benchmark 主要是组件化 Rust bench；Rolldown 同时维护 Rust 与 Node benchmark。两者的 release 工作流都按 apps/crates/N-API/npm 等产物边界拆开，并抽取可复用的多平台构建工作流。[Oxc benchmark/release](https://github.com/oxc-project/oxc/tree/8da040290cc021376d64e5621e1da4fe214bd14f/.github/workflows)、[Rolldown benchmark/release](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/.github/workflows)
6. **公开文档与实现文档的归属取决于发布方式。** Rolldown 在同仓库区分站点 `docs/` 和实现主题 `internal-docs/`；Oxc 当前仓库保留根级架构/维护文档，而完整网站在单独仓库联调。[Rolldown docs](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs)、[Oxc `justfile`](https://github.com/oxc-project/oxc/blob/8da040290cc021376d64e5621e1da4fe214bd14f/justfile)

## 上游文档与实际树的一个差异

Rolldown 的 `docs/development-guide/repo-structure.md` 仍写着 `/web/docs`，但本次固定提交的实际顶层树是 `docs/`，且没有 `web/`。因此涉及具体路径时，本文以固定提交的 Git 树和 workspace 清单为准。[结构说明](https://github.com/rolldown/rolldown/blob/b9823050bc658ef65105148ea0504d4fbda7fa4c/docs/development-guide/repo-structure.md)、[固定提交顶层树](https://github.com/rolldown/rolldown/tree/b9823050bc658ef65105148ea0504d4fbda7fa4c)
