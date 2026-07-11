---
layout: page
outline: false
aside: false
sidebar: false
pageClass: page-playground
---

<script setup lang="ts">
import FontPlayground from '../.vitepress/components/FontPlayground.vue'
</script>

<ClientOnly>
  <FontPlayground locale="zh" />
</ClientOnly>

## CSS Unicode 范围

选择 CSS 输出后，可选的 **CSS Unicode 范围** 会向生成的 `@font-face` 规则添加
`unicode-range` 描述符。请输入以逗号分隔的值，例如
`U+0020-007E, U+4E00-9FFF`。

**要保留的字符** 决定子集中实际保留哪些字形。Unicode 范围不会增减字形；它只会
告诉浏览器在匹配文本时，哪些码点应使用这个生成的字体。

## Unicode 分片交付

选择 CSS 输出后，可在 **Unicode 分片交付** 中启用 Latin、CJK 预设，或填写自定义的
逗号分隔范围。每个启用的分片都会生成独立的字体文件与对应 `unicode-range` 的
`@font-face` 规则，例如 `roboto-latin.woff2` 与 `roboto-cjk.woff2`。

不勾选任何分片时，输出与原有的单字体流程保持一致。输入的字符会保留在每个启用的
分片中，而范围会补充该分片的交付覆盖范围。
