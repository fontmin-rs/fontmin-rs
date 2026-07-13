---
layout: page
outline: false
aside: false
sidebar: false
pageClass: page-playground
---

<script setup lang="ts">
import FontPlayground from './.vitepress/components/FontPlayground.vue'
</script>

<ClientOnly>
  <FontPlayground />
</ClientOnly>

<div
  class="vp-doc mx-auto mb-12 w-full max-w-3xl px-5 sm:px-8"
  data-testid="playground-documentation"
>

## CSS Unicode ranges

When CSS output is selected, the optional **Unicode ranges for CSS** field
adds a `unicode-range` descriptor to the generated `@font-face` rule. Enter
comma-separated values such as `U+0020-007E, U+4E00-9FFF`.

The **Characters to keep** field decides which glyphs are retained in the
subset. Unicode ranges do not add or remove glyphs; they tell browsers which
code points should use this generated font face when matching text.

## Unicode delivery slices

When CSS output is selected, **Unicode delivery slices** can generate separate
font files for the Latin and CJK presets, or for a custom comma-separated range
list. Each enabled slice creates its own `@font-face` rule with the matching
`unicode-range`; for example, `roboto-latin.woff2` and `roboto-cjk.woff2`.

Leave every slice unchecked to preserve the existing single-font output. The
characters you enter are included in each enabled slice, while its range adds
the requested delivery coverage.

</div>
