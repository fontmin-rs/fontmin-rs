# 故障排查

首先记录已安装版本、Node.js 版本、操作系统、CPU 架构、请求的 runtime、输入格式和完整
诊断。`fontmin-rs inspect <font> --json` 可以生成机器可读的输入摘要。

## Native binding 无法加载

发布的 `fontmin-rs` 将 native 平台包声明为 optional dependency。确认安装时没有禁用
optional dependency，并检查当前平台是否在[支持矩阵](./support.md)中。

```sh
pnpm install
node --input-type=module -e \
  "import{readFileSync}from'node:fs';import{inspect}from'fontmin-rs';console.log(inspect(readFileSync(process.argv[1])))" \
  path/to/font.ttf
```

可以接受 WASM 回退时使用 `runtime: "auto"`；要求 native 缺失时立即失败则使用
`runtime: "native"`。不要在不同操作系统、CPU 架构、libc 变体或 Node-API 平台包之间
复制 `.node` 文件。

## WASM 回退失败

Node 包依赖 `@fontmin-rs/wasm`。请从 lockfile 重新安装，并检查 bundler 是否排除了
`.wasm` 资源。浏览器代码应直接导入 `@fontmin-rs/wasm` 并使用异步、纯内存 API；
文件路径和 Node plugin hook 不可用。

## 找不到配置或配置被拒绝

请在正确项目目录运行 CLI，或显式传入 `--config`。执行 TypeScript、MTS、MJS、CJS
配置要求 Node.js 22 或更高版本。未知字段以及 runtime/fallback 冲突会按设计返回错误。
请对照[配置参考](./guide/config.md)检查解析后的结构。

## 字体被拒绝

先分别运行 `inspect` 和 `coverage`。得到稳定的 `invalid-font`、`unsupported` 或缺字
诊断后，不要盲目换输出格式重试。分享机密字体前应先最小化；可能暴露 parser 故障的
malformed input 可按[安全策略](https://github.com/fontmin-rs/fontmin-rs/security/policy)私密报告。

## 输出与经典 Fontmin 不同

应比较解析后的 metadata、请求 glyph coverage、CSS 语义和文件命名；字节完全一致不属于
兼容性承诺。[迁移指南](./guide/migration.md)列出有意保留的 API 差异，
[性能策略](./benchmarks.md)说明代表性的配对 benchmark。

## 报告可复现问题

普通 bug 请提交 GitHub issue，并附上最小可再分发字体或 synthetic reproducer，以及本页
开头列出的环境信息。疑似安全问题请使用私密漏洞报告，不要创建公开 issue。
