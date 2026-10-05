# FeatureVariations：范围评估与验收入口

日期：2026-10-05。本文记录代码审查、固定来源字体检查与本地 Node/HarfBuzz
实验；不宣称已实现 FeatureVariations 保留，也不涉及版本或发布安排。

## 结论

先保持 `keepLayout: 'preserve'` 对 FeatureVariations 的拒绝行为。
真正开放支持需要同时解决替代 feature/lookup 的重映射、GSUB 字形闭包、
变量轴条件和 GPOS 定位语义；仅保留 GSUB/GPOS v1.1 的字节或删除前置检查会
返回可解析但显示错误的字体。已找到并验证 OFL-1.1 的真实 Recursive 字体，
可作为下一阶段的 GSUB 主样本。本次没有验证到含 **GPOS FeatureVariations**
的真实字体，不能以 GSUB 成功代替 GPOS 验收。

## 当前实现为何拒绝

- [`ensure_layout_can_be_preserved`](../../../crates/fontmin_subset/src/lib.rs)
  在计划解析阶段检查 GSUB、GPOS：major=1、minor>=1、FeatureVariations
  offset 非零即返回 `fontmin::config`。当前消息为
  `keepLayout preserve cannot retain GSUB FeatureVariations; use conservative or drop`
  （GPOS 使用对应表名）。
- [`oxifont OTL`](../../../vendor/oxifont-subset/src/otl.rs) 明确说明重写
  Script/Feature/Lookup 后原 feature index 会失效，因此输出 v1.0 并删除
  FeatureVariations；[`GPOS`](../../../vendor/oxifont-subset/src/otl_gpos.rs)
  使用同样策略。重写还会改变 lookup index 与 glyph ID。
- 调查时同一 OTL 实现没有 GSUB closure pass；本轮随后补齐选定普通
  Single/Multiple/Alternate/Ligature/Extension 的闭包。FeatureVariations
  替代 lookup 与 contextual dispatch 仍不在该闭包范围。
- `preserve` 另有事后检查，拒绝丢失整个 GDEF/GPOS/GSUB 表或丢弃 contextual
  subtables。现有 `with_gsub_feature_variations` 测试只修改表头，覆盖的是拒绝
  分支，不是一个合法 FeatureVariations 的解析、改写或 shaping 测试。
- CFF2→TTF 是另一入口：
  [`validate_cff2_layout_tables`](../../../crates/fontmin_otf/src/sfnt.rs)
  拒绝 GSUB/GPOS FeatureVariations，错误属于 unsupported 格式范围。
  两个入口的保护不能仅通过修复其中一个而同时移除。

## 已验证的真实 GSUB 样本

| 项目 | 固定值 |
| --- | --- |
| 字体 | Recursive `[CASL,CRSV,MONO,slnt,wght]`，TrueType glyf |
| 上游 commit | `google/fonts@9710da1eacb3be272583c3224dcb70f9da6eadbb` |
| 大小 | 2,379,132 bytes |
| SHA-256 | `653221ca467f4732fe6856ac493f6c409e9f56a7674abe36b2364acc89796f7c` |
| 许可 | [同一 commit 的 OFL.txt](https://github.com/google/fonts/blob/9710da1eacb3be272583c3224dcb70f9da6eadbb/ofl/recursive/OFL.txt) |
| 许可文件 SHA-256 | `f9f539cf7549bd417159dbdb9c400943a5b60a7366c2c6fbde9f095173d82479` |
| GSUB | v1.1，FeatureVariations offset=14，11 个 records，全部为 Condition format 1 |
| GPOS | v1.0，无 FeatureVariations |
| 其他相关表 | `avar`, `fvar`, `gvar`, `HVAR`, `MVAR`, `GDEF` |

[固定二进制下载](https://raw.githubusercontent.com/google/fonts/9710da1eacb3be272583c3224dcb70f9da6eadbb/ofl/recursive/Recursive%5BCASL,CRSV,MONO,slnt,wght%5D.ttf)。
二进制 `fvar` 实测顺序是 `MONO [0,0,1]`、`CASL [0,0,1]`、
`wght [300,300,1000]`、`slnt [-15,0,0]`、`CRSV [0,0.5,1]`，
每组三值依次为最小值、默认值、最大值。FeatureVariations 条件涉及
MONO、slnt、CRSV，存在重叠边界，适合验证轴顺序和 first-match。
字体作者也明确说明 CRSV 控制 Roman/Cursive 字形、与 slnt 交互：
[ArrowType 的 Recursive 介绍](https://www.arrowtype.com/custom/recursive)。

2026-10-05 在本轮 GSUB/GPOS 修复前，使用工作区现有
`packages/fontmin/dist/index.mjs` native binding，
文本 `ag`、`retainGids: true`，在本机 HarfBuzz 14.5.1 得到：

| 操作 | 坐标 | GID | horizontal advances |
| --- | --- | --- | --- |
| 原字体 | CRSV=0，其余默认 | 250, 311 | 590, 600 |
| 原字体 | CRSV=1，其余默认 | 993, 1054 | 600, 600 |
| conservative 子集 | CRSV=0 | 250, 311 | 600, 600 |
| conservative 子集 | CRSV=1 | 250, 311 | 600, 600 |
| preserve 子集 | 任意运行坐标 | 拒绝：`fontmin::config` | 无输出 |
| `instantiateFont`，只指定 CRSV=1 | 输出为静态字体 | 250, 311 | 590, 600 |
| `reduceVariationSpace`，pin CRSV=1 | 其他轴仍可变、取默认 | 993, 1054 | 600, 600 |

该修复前 conservative 的 report 只有 old GID 0、250、311 的映射，
`droppedContextSubtables=107`；不会自动保留 993、1054。
advance 差异还说明不能把所有现有差异归因于 FeatureVariations；
普通定位改写也需要独立的修复和基线。

`instantiateFont` 的实测差异是另一项现有限制：
[`oxifont instance`](../../../vendor/oxifont-subset/src/instance/mod.rs)
原样复制 GSUB/GPOS/GDEF，同时删除 fvar/avar 等表，未将命中的替代 feature
固化进静态输出。本轮仅记录，后续应单独修复或明确拒绝；不要把这个入口用作
FeatureVariations 的正确性 oracle。`reduceVariationSpace` 使用
[已固定的 HarfBuzz 14.3.0 WASM](../../../vendor/harfbuzz-subset-wasm/README.md)，
这里的单轴实验可作为比较基线，但尚未穷尽该入口的所有条件/轴/表组合。

复现方法（把下载文件命名为 `/tmp/recursive.ttf`，输出也写 `/tmp`）：

```sh
rtk proxy node --input-type=module - <<'JS'
import { readFileSync, writeFileSync } from 'node:fs'
import { subsetTtfWithReport } from './packages/fontmin/dist/index.mjs'

const input = readFileSync('/tmp/recursive.ttf')
for (const keepLayout of ['preserve', 'conservative', 'drop']) {
  try {
    const { data, report } = subsetTtfWithReport(input, {
      text: 'ag', keepLayout, retainGids: true,
    })
    writeFileSync(`/tmp/recursive-${keepLayout}.ttf`, data)
    console.log(keepLayout, report.oldToNew, report.droppedContextSubtables)
  } catch (error) {
    console.log(keepLayout, error.code, error.message)
  }
}
JS
rtk proxy hb-shape /tmp/recursive.ttf ag --variations=CRSV=1 --shapers=ot --script=Latn --language=en --no-glyph-names --output-format=json
rtk proxy hb-shape /tmp/recursive-conservative.ttf ag --variations=CRSV=1 --shapers=ot --script=Latn --language=en --no-glyph-names --output-format=json
```

## 语义与建议范围

[OpenType common layout](https://learn.microsoft.com/en-us/typography/opentype/spec/chapter2)
定义 FeatureVariations 可替换 GSUB 或 GPOS 的 feature 所引用的 lookup 集合；
它与 GDEF ItemVariationStore/GPOS VariationIndex 的连续数值调整是不同机制。
因此验收既要比较字形，又要比较定位。

第一阶段建议只承诺合法 v1.1 表和 Condition format 1，并显式拒绝其他
不能无损处理的 condition/substitution 格式。参考本仓库使用版本对应的
[HarfBuzz 条件求值与重写源码](https://github.com/harfbuzz/harfbuzz/blob/4de187dd0a915d13c976fa8bd474c084229f3aab/src/hb-ot-layout-common.hh)：
format 1 使用闭区间；同一 condition set 全部满足才命中；按记录顺序采用
首个匹配项。当前该上游文件也含 condition 2–5 的求值代码与 subset TODO，
所以不能把“复用 HarfBuzz”直接解释成所有 condition 都有完整子集支持。

必须覆盖以下变换：

1. 普通子集保持变量空间：为保留的 feature 收集所有可达替代 lookup，完成
   GSUB closure，再统一改写 GID、lookup index、feature index；保留记录顺序。
2. features/scripts/languages 筛选：删除 feature 时同步改写其 substitution；
   默认 feature 没有 lookup、但替代 feature 有 lookup 的情况也必须保留。
3. 单轴 pin / 范围收窄：在 fvar 用户单位、归一化坐标、avar 映射之间正确换算，
   重排 axis index、裁剪条件，处理新默认值与永真/永假条件，不能直接复用旧阈值。
4. 全轴实例化：求值后固化命中 feature，移除无意义的 FeatureVariations，
   同时兑现 GPOS/GDEF 的数值变化；保持原入口既有 GID 身份契约。
5. GPOS 替代 lookup：验证 x/y advances、x/y offsets、kerning、mark anchors，
   保证 GDEF variation store 的引用有效。不能只检查表名或 checksum。

## 四个公共入口的未来契约

| 入口 | 必须一致的行为 |
| --- | --- |
| Rust `subset_ttf*` / subset plan | `Preserve` 成功意味着保留选定布局在剩余变量空间的语义；不支持时保留明确错误；plan 执行与直接调用一致 |
| Node `subsetTtf*`、glyph plugin | 沿用 `keepLayout`、report/GID 映射、稳定 diagnostic code；同步/异步与文件管线一致 |
| browser WASM 对应 API | 同样的 feature/script/language/axis 行为与 diagnostic code；实际 WASM 输出需进入 shaping 验收 |
| Rust/Node CLI | JSON/模块配置的 glyph `keepLayout` 与 API 一致；拒绝时非零退出且不留下成功产物；当前直接 `subset` 命令没有 `--keep-layout`，测试不应杜撰该参数 |

`conservative` 和 `drop` 保持明确的现有策略，不借这次范围评估改变默认值。
如果后续加入 dropped FeatureVariations 统计，应作为独立、跨 Rust/Node/WASM/CLI
一致的报告字段设计，不能复用 `droppedContextSubtables` 来暗示它已计数。

## 验收方案

- 固定 Recursive 下载 URL、完整 SHA-256、OFL 与作者信息；先接入现有 fixture
  inventory 校验。若需要缩小样本，记录原始 hash、工具版本与完整推导命令，
  并证明推导没有删掉条件边界、替代字形或关键布局；不能用当前会丢 FV 的
  conservative 子集生成正向基准。
- 真实 GSUB：`ag` 至少测 CRSV=0/0.5/1、MONO=0/0.5/1、slnt=0/-15，
  加每个实际 condition 阈值及两侧的 F2Dot14 相邻值，并保留 record 优先级。
  使用 source→output GID 映射后比较 glyph sequence、cluster、advance、offset。
- 格式边界：单独构造合法且可审计的 GSUB/GPOS format-1 小字体，覆盖空/永真
  condition set、重叠记录、空默认 feature、feature 过滤、lookup 重排、坏 offset、
  越界 axis/feature/lookup index。真实 GPOS FeatureVariations 样本仍是待补证据。
- GPOS：至少一对真实 kerning 和 mark 场景，另有可控的 GPOS feature 条件切换；
  仅 GID 一致或只有 GSUB 样本不能关闭该项。
- 轴变换：单轴 pin、多轴 pin、区间裁剪、改变默认值、全轴实例化，与原字体
  对应用户坐标比较；默认轴和 avar 非线性映射必须进入矩阵。
- 四入口输出统一交给固定版本 HarfBuzz `ot` shaper；固定 direction、script、
  language、features、坐标和 UPEM，禁用环境默认值的干扰。无论输出是否保留 GID，
  都通过 report 映射比较，不把重编号当作失败。
- 结构层同时检查可解析、校验和、所有引用在界内；选定文本的渲染再作为额外
  视觉回归。不可用“存在 GSUB/GPOS 表”代替语义验收。

## 可复用样本与工具边界

现有 [Source Serif 4 CFF2 fixture](../../../fixtures/fonts/manifest.json)
已具备固定下载来源、OFL-1.1 与 SHA-256
`867b73c6a954a4a64616906d179f94572a748790a1d022ebeeff07f56ea0221a`；
实测 GSUB、GPOS 均 v1.0，wght=200..900/default 400、opsz=8..60/default 20。
它适合 CFF2 轮廓/轴回归，不能作为 FeatureVariations 正向样本。
本轮并行检查还发现其 `office AV` 普通布局缺口：`ffi` 原 GID 422 未由子集
闭包保留，A 的 advance 从 545 变成 664，preserve 拒绝 11 个 contextual
subtables。上述为本轮修复前基线。本轮新增真实字体回归已确认普通 GSUB
闭包会保留 GID 422，并独立修复 GPOS PairPos/Device/VariationIndex 重写；
生产门禁继续检查指定文本及坐标的 shaping/rendering。这些修复没有开放
FeatureVariations 或保证所有 contextual 布局。

普通闭包遵循 feature/script/language 过滤；`keepLayout: 'drop'` 不扩张
布局字形。`dropTables: ['GSUB']` 仍沿用后处理删除策略，可能先保留额外
GSUB 字形再删表；本轮没有新增另一套表级闭包控制 API。为限制恶意字体
用共享偏移放大内存，lookup/subtable 去重，并对累计规则输入/输出引用设
1,000,000 上限；超限返回 invalid-font 错误，不会静默少保字形。

本次还下载、解析了 Google Fonts 的两个小型真实 COLRv1 构建，供需要更小
样本时选择；它们没有 GSUB/GPOS/fvar，不能覆盖变量布局或变量色彩：

| 固定 commit `googlefonts/color-fonts@0046ea4c3b69e9fbbe464c2594816894e3aa5e4b` 下文件 | bytes | SHA-256 |
| --- | --- | --- |
| [twemoji_smiley-glyf_colr_1.ttf](https://github.com/googlefonts/color-fonts/blob/0046ea4c3b69e9fbbe464c2594816894e3aa5e4b/fonts/twemoji_smiley-glyf_colr_1.ttf) | 7,420 | `b462e4de616a38979b053a49b0b5a5f2dd72b6d5f55be43c5b1eb812fd438dbc` |
| [twemoji_smiley-cff2_colr_1.otf](https://github.com/googlefonts/color-fonts/blob/0046ea4c3b69e9fbbe464c2594816894e3aa5e4b/fonts/twemoji_smiley-cff2_colr_1.otf) | 5,324 | `217bfb5a2876029afa9f52affe9810f398ba48574518fc228ee8966640af8ae3` |

该构建仓库代码是 Apache-2.0，但 [固定构建配置](https://github.com/googlefonts/color-fonts/blob/0046ea4c3b69e9fbbe464c2594816894e3aa5e4b/config/twemoji_smiley-glyf_colr_1.toml)
引用 Twemoji SVG；其 [固定子模块的许可说明](https://github.com/twitter/twemoji/blob/8017ebd412a6293993aa5c0709f0e035714b3cdd/README.md#license)
指定图形为 CC-BY-4.0，不能把代码许可证当成字体素材许可。本轮已选的 Noto
COLRv1 样本可继续作为主回归字体，无需为这两个候选额外扩大 fixture 集。

本机 `/opt/homebrew/bin/hb-shape`、`hb-view` 均为 14.5.1；
`hb-shape` 支持 JSON GID/cluster/position 和 `--variations`，`hb-view` 已成功
输出 Recursive CRSV=1 与 smiley COLRv1 的 PNG。默认 Python 没有 fontTools，
PATH 没有 `ots-sanitize`；本次结构检查使用 Python 标准库读取 sfnt/fvar/FV，
没有增加运行时或构建依赖。CI 应显式提供固定 HarfBuzz 工具版本，不能依赖
开发机 PATH；渲染还要锁定 rasterizer/backend，避免跨平台像素误差掩盖语义结果。
