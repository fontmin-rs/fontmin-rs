# Oxfmt npm 配置 JSON Schema 调研

## 调研范围

- 调研日期：2026-07-29。
- Oxc 基准：`oxc-project/oxc` 的 `main` 提交
  [`1b57f783fb75617afe36b93a764feeed2fb83c92`](https://github.com/oxc-project/oxc/tree/1b57f783fb75617afe36b93a764feeed2fb83c92)。
- Oxc 官方网站基准：`oxc-project/website` 的 `main` 提交
  [`d3bc1574d16ff06c754b93695d4928c88f6a6eb1`](https://github.com/oxc-project/website/tree/d3bc1574d16ff06c754b93695d4928c88f6a6eb1)。
- 资料只取自上述官方仓库的源码、测试、工作流和官方使用文档。
- 本仓库已有 `docs/superpowers/research/` 约定，因此本文沿用日期前缀放在该目录，不进入公开文档站导航。

## 结论摘要

Oxfmt 并不手写两套彼此独立的配置定义。它把 Rust
`Oxfmtrc`/`FormatConfig` 及其 `JsonSchema` 派生信息作为配置模型来源，
生成并提交 `npm/oxfmt/configuration_schema.json`，再从该 JSON Schema
生成公开的 TypeScript 配置类型。生成器还加入了面向编辑器的 JSONC 和
Markdown 描述扩展。

npm 包通过 `files` 把 Schema 放入发布 tarball，但当前没有在 `exports`
中声明 `./configuration_schema.json` 子路径。官方用法是让
`.oxfmtrc.json`/`.oxfmtrc.jsonc` 的 `$schema` 指向
`./node_modules/oxfmt/configuration_schema.json`；这是文件系统路径约定，
不是 JavaScript 模块子路径导入。

一致性由四层机制保障：

1. Rust 测试比较“现场生成结果”和仓库中提交的 Schema；
2. 配置参考文档由同一 Rust 类型生成并做 snapshot；
3. CLI snapshot 覆盖 `--init` 是否正确加入本地 `$schema`；
4. 发布工作流检查 `files` 中的文件存在，并执行 npm 发布 dry-run。

## Schema 来源与生成链路

### 权威配置模型

配置模型位于
[`apps/oxfmt/src/core/oxfmtrc.rs`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/apps/oxfmt/src/core/oxfmtrc.rs#L11-L55)。
`Oxfmtrc`、`OxfmtOverrideConfig`、`FormatConfig` 以及相关枚举/嵌套类型同时
派生 Serde 和 `schemars::JsonSchema`。Rust doc comment、Serde 的字段重命名、
可选字段和 `#[schemars(...)]` 属性因此共同决定运行时反序列化和 Schema。

生成产物提交在
[`npm/oxfmt/configuration_schema.json`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/npm/oxfmt/configuration_schema.json)。
该文件声明 JSON Schema Draft 7，并包含 `properties`、`definitions`、字段说明
及编辑器扩展；它是发布产物，不是另一个手工维护的配置模型。

### 通用生成器

共享生成逻辑位于
[`tasks/website_common/src/schema_json.rs`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/tasks/website_common/src/schema_json.rs#L5-L46)：

- 使用 `SchemaSettings::draft07()`；
- 关闭 `Option<T>` 自动追加 `null`，因为可选字段已由 `required` 与否表达；
- 在根 Schema 加入 VS Code JSON language service 使用的非标准
  `allowComments` 和 `allowTrailingCommas`；
- 把每个标准 `description` 复制为非标准 `markdownDescription`，改善 VS Code
  hover 和补全说明。

Oxfmt 的适配层位于
[`tasks/website_formatter/src/json_schema.rs`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/tasks/website_formatter/src/json_schema.rs#L1-L42)。
它把 `Oxfmtrc` 交给共享生成器，并提供 JSON Schema 输出和 Markdown
配置参考输出。

### 更新与派生顺序

仓库的
[`justfile`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/justfile#L288-L311)
暴露两个独立命令：

1. `just formatter-schema-json` 运行
   `cargo run -p website_formatter schema-json`，重写
   `npm/oxfmt/configuration_schema.json`；
2. `just formatter-config-ts` 从上述 Schema 生成
   `apps/oxfmt/src-js/config.generated.ts`。

第二步由
[`apps/oxfmt/scripts/generate-config-types.ts`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/apps/oxfmt/scripts/generate-config-types.ts#L8-L30)
调用 `json-schema-to-typescript`。因此依赖方向是：

```text
Rust Oxfmtrc / FormatConfig
  ├─→ committed configuration_schema.json
  │    └─→ generated public TypeScript config types
  └─→ generated website config reference
```

网站的配置参考同样由 `website_formatter schema-markdown` 生成；`just website`
会把结果写入独立的 Oxc 网站仓库。Schema、TypeScript 类型和配置参考因而从
同一 Rust 模型派生，而不是分别录入。

## npm 包中的包含与暴露方式

[`npm/oxfmt/package.json`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/npm/oxfmt/package.json#L25-L43)
有两个需要区分的事实：

- `files` 明确包含 `configuration_schema.json`，所以 npm 打包时会把它放在
  `oxfmt` 包根目录；
- `exports` 只声明包根入口和 `./package.json`，没有声明
  `./configuration_schema.json`。

因此 Oxfmt 当前提供的是“已发布的包内文件”，不是
`oxfmt/configuration_schema.json` 这个 Node package exports 子路径。
其自身代码也没有通过 `import`/`require` 读取该文件，而是检查
`node_modules/oxfmt/configuration_schema.json` 这个文件系统路径：
[`apps/oxfmt/src-js/cli/migration/shared.ts`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/apps/oxfmt/src-js/cli/migration/shared.ts#L8-L30)。

发布时，
[`release_apps.yml`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/.github/workflows/release_apps.yml#L591-L633)
先组装 JS 和各平台 N-API 产物，再运行
[`check-npm-packages.js`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/.github/scripts/check-npm-packages.js#L35-L60)。
该脚本逐项检查 `files` 中声明的路径存在，并执行 `pnpm publish --dry-run`。
所以漏掉 Schema 文件会在正式发布前失败。

## 配置文件与编辑器关联

Oxc 官方
[`Configuration` 文档](https://github.com/oxc-project/website/blob/d3bc1574d16ff06c754b93695d4928c88f6a6eb1/src/docs/guide/usage/formatter/config.md#L59-L74)
明确把 `$schema` 用于编辑器校验和自动补全：

```json
{
  "$schema": "./node_modules/oxfmt/configuration_schema.json"
}
```

Oxc 仓库自身也在
[`oxfmtrc.jsonc`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/oxfmtrc.jsonc#L1-L3)
使用同一模式，只是从 monorepo 根目录直接指向
`./npm/oxfmt/configuration_schema.json`。

`oxfmt --init` 会先检查当前工作目录的
`node_modules/oxfmt/configuration_schema.json`：

- 文件存在时，生成的配置包含相对 `$schema`；
- 例如仅通过 `npx` 临时执行、项目本地没有 Schema 文件时，省略 `$schema`，
  避免产生悬空路径。

这两个行为分别由
[`init_with_schema` snapshot](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/apps/oxfmt/test/cli/init_with_schema/0.snap.md#L31-L40)
和
[`init` snapshot](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/apps/oxfmt/test/cli/init/0.snap.md#L29-L37)
固定。

对 TypeScript 配置，官方文档使用从包根导出的 `defineConfig()` 来获得类型
检查和编辑器补全，而不是使用 JSON Schema。也就是说，Oxfmt 将 JSON/JSONC
的 `$schema` 和 TS 配置的类型系统视为两条互补入口。

本次固定提交的 Oxc 仓库及官方配置文档只展示显式 `$schema` 关联，没有发现
通过 Schema Store 文件名匹配自动关联 `.oxfmtrc.json(c)` 的实现。

## 测试与发布保障

| 层级 | 保障内容 | 一手来源 |
| --- | --- | --- |
| Schema 漂移 | `test_schema_json` 重新从 `Oxfmtrc` 生成 Schema，与提交的 `npm/oxfmt/configuration_schema.json` 做规范化换行后的精确比较；不一致时提示运行 `just formatter-schema-json`。 | [`tasks/website_formatter/src/json_schema.rs`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/tasks/website_formatter/src/json_schema.rs#L10-L24) |
| 配置参考漂移 | 同一 `Oxfmtrc` 生成 Markdown，并通过 `insta` snapshot 比较。 | [`tasks/website_formatter/src/json_schema.rs`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/tasks/website_formatter/src/json_schema.rs#L26-L42) |
| `$schema` 初始化 | CLI snapshot 同时覆盖本地 Schema 存在和不存在两种情况。 | [`init_with_schema`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/apps/oxfmt/test/cli/init_with_schema/0.snap.md#L8-L40)、[`init`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/apps/oxfmt/test/cli/init/0.snap.md#L8-L37) |
| CI 执行 | 主 CI 在 Linux 运行 workspace `cargo test --all-features`，覆盖 `website_formatter` 的生成一致性测试；Oxfmt 的 N-API job 另行运行应用测试并要求工作树无生成差异。 | [`.github/workflows/ci.yml`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/.github/workflows/ci.yml#L17-L35)、[Oxfmt job](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/.github/workflows/ci.yml#L287-L332) |
| npm 发布内容 | 发布前逐项检查 `files` 路径存在，并执行发布 dry-run。 | [`check-npm-packages.js`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/.github/scripts/check-npm-packages.js#L35-L60)、[`release_apps.yml`](https://github.com/oxc-project/oxc/blob/1b57f783fb75617afe36b93a764feeed2fb83c92/.github/workflows/release_apps.yml#L622-L633) |

未发现专门断言 `exports` 可解析 Schema 子路径的测试；这与当前 package
manifest 没有导出该子路径一致。也未发现用一组正例/反例配置直接运行通用
JSON Schema validator 的测试；其主要语义保障来自同一个 Rust 类型既参与
Serde 反序列化、又参与 Schema 生成。

## 对 `fontmin-rs` 的可迁移结论

以下是基于上游事实、结合本仓库现状的实现建议，不代表 Oxfmt 已采用这些完全
相同的选择。

### 1. 先固定 JSON Schema 的配置边界

本项目的 TypeScript `FontminConfig` 可包含 `Uint8Array`、自定义插件和函数，
而 `fontmin.config.json/jsonc` 必须是 JSON 可表达的数据；Rust CLI 与 Node
配置还存在 `otf`、`delivery`、`runtime` 等差异。因此不能未经裁剪地把完整
TypeScript 类型机械转换成 JSON Schema。

建议把 Schema 明确定义为 `fontmin.config.json/jsonc` 的“可序列化 npm/CLI
配置契约”，并在 title/description 中说明它不描述可执行的 TS/MJS 配置和
自定义插件函数。实现前需要决定它是：

- Rust 与 Node 都接受的公共子集；或
- npm `fontmin-rs` CLI 实际接受的 JSON/JSONC 超集，并在字段说明中标注运行时。

这一选择应成为唯一生成源；否则 Schema、Rust Serde 类型和 TypeScript
`FontminConfig` 会重新产生三份漂移。

### 2. 分离“进入 tarball”和“Node 子路径导出”

可以沿用 Oxfmt 的包根文件名 `configuration_schema.json`，并至少加入
`package.json#files`。如果本项目还希望把 Schema 当作稳定的 package
subpath API，则应额外声明：

```json
{
  "exports": {
    "./configuration_schema.json": "./configuration_schema.json"
  }
}
```

这是对 Oxfmt 当前方案的增强：`files` 决定发布内容，`exports` 决定 Node
package subpath 契约，两者职责不同。配置中的
`./node_modules/fontmin-rs/configuration_schema.json` 是编辑器文件路径，
本身不依赖 `exports`，但显式导出更利于工具通过包解析器定位 Schema。

### 3. 初始化、文档和测试应同时落地

推荐的完整闭环是：

1. `fontmin-rs init` 在本地安装的 Schema 文件存在时，将
   `"$schema": "./node_modules/fontmin-rs/configuration_schema.json"`
   放在生成配置首项；
2. `docs/guide/config.md` 的首个 JSONC 示例说明 `$schema` 提供校验、补全和
   hover 文档，TS 配置继续推荐 `defineConfig()`；
3. 生成一致性测试确保配置模型变化时必须提交新 Schema；
4. validator 测试至少覆盖文档示例和未知字段/错误枚举等反例；
5. package smoke test 检查 `pnpm pack` 后 Schema 确实存在，且声明了
   `exports` 时可通过包解析器定位；
6. CLI 测试覆盖本地 Schema 存在与不存在，避免 `npx` 场景写出悬空路径。

### 4. 建议的实施顺序

1. 明确 Schema 覆盖的可序列化配置类型及唯一来源；
2. 加入生成器、提交产物和漂移测试；
3. 加入 `files`、可选的显式 `exports` 与 package smoke test；
4. 更新 `init` 模板及其测试；
5. 更新配置文档，并用 Schema validator 校验文档中的 JSON/JSONC 示例。

这个顺序把配置语义、npm 分发和编辑器体验拆成可独立验证的边界，也避免先发布
一个尚未定义清楚的 Schema 契约。
