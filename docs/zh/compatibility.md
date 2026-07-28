# 兼容性证据

`1.0` 契约除单元测试、集成测试、一致性测试和包内容检查外，还会通过独立消费者项目
验证。这些项目会把候选 tarball 安装到临时目录，并且只使用公开入口。

## 消费者项目

| 项目                | 验证边界                                                             |
| ------------------- | -------------------------------------------------------------------- |
| 独立 CLI 与 Node.js | 打包后的命令、native API、native pipeline、自动 WASM 回退、强制 WASM |
| 浏览器字体加载      | 打包后的 Node pipeline、生成 CSS/WOFF/WOFF2、`FontFaceSet` 加载      |
| 独立浏览器 WASM     | 打包后的 `@fontmin-rs/wasm`、内存检查与优化、`FontFace` 加载         |

安装 Chromium 后运行 workspace tarball 报告：

```sh
pnpm --filter fontmin-rs exec playwright install chromium
pnpm run compatibility:check
```

报告会记录 package 来源、精确版本、Node.js 版本、操作系统、架构、浏览器、每个项目
覆盖的接口和结果。CI 会把 `compatibility/current.json` 上传为
`compatibility-report` artifact；任一项目未通过都会阻断发布。

## 已发布的候选版本

同一组项目也可以从 npm 安装精确版本：

```sh
node scripts/compatibility-report.mjs \
  --registry-version 1.0.0-rc.1 \
  --output compatibility/1.0.0-rc.1.json
```

registry 模式验证用户实际收到的 package metadata、可选 native 依赖、WASM 依赖、
命令入口和浏览器资产。稳定版提升前会提交一份经过审阅的 RC 报告。

报告冻结语义、诊断、生成文件名和浏览器可加载性；编码器字节级完全相同不属于兼容性
承诺。
