# 命令行

安装 `fontmin-rs` 后会暴露同名 bin：

```sh
fontmin-rs --help
```

本仓库开发时也可以直接运行 Rust CLI：

```sh
cargo run -p fontmin_app -- inspect fixtures/fonts/ttf/roboto-regular.ttf --json
```

## init

在当前目录创建初始 `fontmin.config.jsonc`。

```sh
fontmin-rs init
```

如果 `fontmin.config.jsonc` 已存在，该命令会拒绝覆盖。

## subset

按文本裁剪 TTF 字体。

```sh
fontmin-rs subset fixtures/fonts/ttf/roboto-regular.ttf \
  --text "Hello" \
  --output build/roboto-subset.ttf
```

可用参数：

| 参数                    | 说明                     |
| ----------------------- | ------------------------ |
| `INPUT`                 | 输入字体路径             |
| `-o, --output <OUTPUT>` | 输出 TTF 路径            |
| `-t, --text <TEXT>`     | 需要保留的文本           |
| `--text-file <FILE>`    | 从文件读取需要保留的文本 |
| `--unicodes <LIST>`     | 逗号分隔的 Unicode 码点  |
| `-b, --basic-text`      | 额外保留基础文本字符集   |

## convert

在支持的字体格式之间转换。

```sh
fontmin-rs convert fixtures/fonts/ttf/roboto-regular.ttf \
  --format woff2 \
  --output build/roboto.woff2
```

常见目标格式：

| 格式    | 用途                                    |
| ------- | --------------------------------------- |
| `woff2` | 现代浏览器首选 Web 字体格式             |
| `woff`  | Web 字体回退格式                        |
| `eot`   | 旧版 IE 兼容格式                        |
| `svg`   | SVG font 输出                           |
| `ttf`   | 将静态 CFF OTF，或 WOFF/EOT，转换回 TTF |

对于 CFF2 可变字体，可以重复使用 `--variation TAG=VALUE` 选择用户空间实例：

```sh
fontmin-rs convert fixtures/fonts/otf/source-serif-4-variable-roman.otf \
  --format ttf \
  --variation wght=700 \
  --variation opsz=14 \
  --output build/source-serif-4.ttf
```

输出是静态 TTF，不包含 CFF2 或 variation 表，也不会保留 Type 2 hinting。

## build

`build` 是批量处理入口，适合项目脚本和 CI。

```sh
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf \
  -o build \
  --text "Hello" \
  --preset modern-web \
  --font-family Roboto
```

使用 `--formats` 可精确控制输出格式；使用 `--preset modern-web` / `--preset compat` 可选择常见字体输出组合。对多个 SVG icon 输入使用 `--preset iconfont`，会输出 `iconfont.ttf` 和 `iconfont.css`。

静态 CFF OTF 与 CFF2 可变 OTF 输入会在 Web 管线子集化和转换之前规范化为静态 TTF。对于 CFF2，可重复使用 `--variation` 选择实例：

```sh
fontmin-rs build fixtures/fonts/otf/source-serif-4-variable-roman.otf \
  -o build \
  --preset modern-web \
  --variation wght=700 \
  --variation opsz=14
```

需要生成具名 Unicode 分片时，重复使用 `--delivery-slice`。选择 CSS 输出后，每个分片都会生成对应的字体文件和 `@font-face` 描述符：

```sh
fontmin-rs build fixtures/fonts/ttf/roboto-regular.ttf \
  -o build \
  --text "Hello" \
  --preset modern-web \
  --delivery-slice latin:U+0000-00FF \
  --delivery-slice cjk:U+4E00-9FFF
```

重复使用相同名称会为该分片追加范围。分片名只能包含字母、数字、连字符和下划线。只要提供任意 `--delivery-slice`，就会替换配置文件中声明的分片。

可用参数：

| 参数                             | 说明                                            |
| -------------------------------- | ----------------------------------------------- |
| `INPUT...`                       | 输入字体路径，支持 glob                         |
| `-c, --config <CONFIG>`          | TS、MTS、MJS、CJS、JSON 或 JSONC 配置文件       |
| `-o, --out-dir <OUT_DIR>`        | 输出目录                                        |
| `-t, --text <TEXT>`              | 子集化文本                                      |
| `--text-file <FILE>`             | 从文件读取子集化文本                            |
| `--unicodes <LIST>`              | 逗号分隔的 Unicode 码点                         |
| `-b, --basic-text`               | 额外保留基础文本字符集                          |
| `-d, --deflate-woff`             | 保持 Fontmin 兼容的 WOFF deflate 行为           |
| `-T, --show-time`                | 输出 build 耗时                                 |
| `--silent`                       | 静默可选的 build 耗时输出                       |
| `--cache`                        | 启用 native build 缓存                          |
| `--no-cache`                     | 禁用 native build 缓存                          |
| `--css-glyph`                    | 生成 glyph class CSS 规则                       |
| `--delivery-slice <NAME:RANGES>` | 添加具名 Unicode 分片；重复使用可添加范围或分片 |
| `--variation <TAG=VALUE>`        | 为 OTF 输入选择 CFF2 用户空间轴坐标             |
| `--formats <FORMATS>`            | 逗号分隔的输出格式                              |
| `--preset <PRESET>`              | `modern-web`、`compat` 或 `iconfont`            |
| `--no-original`                  | 移除请求中的原始 TTF 输出                       |
| `--font-family <FONT_FAMILY>`    | CSS 中的字体族名称                              |
| `--font-path <FONT_PATH>`        | CSS 中引用字体文件的路径前缀                    |

Iconfont 示例：

```sh
fontmin-rs build icons/home.svg icons/user.svg \
  -o build/icons \
  --preset iconfont \
  --font-family "Project Icons"
```

## bench

测量单个 TTF 输入的 native subset 性能。

```sh
fontmin-rs bench fixtures/fonts/ttf/roboto-regular.ttf \
  --text-file chars.txt \
  --json
```

不加 `--json` 时输出简短终端摘要；加上 `--json` 后会输出 `operation`、`inputBytes`、`outputBytes` 和 `elapsedMs`，便于脚本或 benchmark harness 消费。

## inspect

读取字体格式和元信息。

```sh
fontmin-rs inspect fixtures/fonts/ttf/roboto-regular.ttf --json
```

不加 `--json` 时输出面向终端阅读的摘要；加上 `--json` 后适合脚本消费。

对于 WOFF2 文件，inspect 会校验 WOFF2 header 和 table directory，并读取 `name`、`head`、`hhea`、`maxp` 等 sfnt metadata tables。`fontmin-rs convert input.woff2 -f ttf -o output.ttf` 可将 WOFF2 解码回 TTF。

## doctor

输出本地环境和 native binding 状态，用于排查安装问题。

```sh
fontmin-rs doctor
```
