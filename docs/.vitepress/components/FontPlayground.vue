<script setup lang="ts">
import { useDropZone, useFileDialog } from '@vueuse/core'
import { computed } from 'vue'
import type { PlaygroundCopy } from '../playground/types'
import { useFontPlayground } from '../playground/useFontPlayground'
import PlaygroundForm from './PlaygroundForm.vue'
import PlaygroundResults from './PlaygroundResults.vue'

const props = withDefaults(
  defineProps<{
    locale?: 'en' | 'zh'
  }>(),
  { locale: 'en' },
)

const englishCopy: PlaygroundCopy = {
  characters: 'Characters to keep',
  charactersHelp: 'characters',
  changeFile: 'Change',
  chooseFile: 'Choose a font file',
  coverage: 'Character coverage',
  coverageComplete: 'Every requested character is supported.',
  coverageMissing: 'Missing code points',
  coverageRequested: 'Requested',
  coverageSupported: 'Supported',
  download: 'Download ZIP',
  downloadAsset: 'Download file',
  downloadZip: 'Download ZIP',
  delivery: 'Unicode delivery slices',
  deliveryCjk: 'CJK (U+4E00-9FFF)',
  deliveryCustom: 'Custom slice',
  deliveryCustomHelp: 'Custom ranges are required when this slice is enabled.',
  deliveryHelp:
    'Optional: each enabled slice creates its own font files and CSS unicode-range descriptor.',
  deliveryLatin: 'Latin (U+0000-00FF)',
  dropFile: 'Drop a font file to replace the current selection.',
  fontFile: 'Font file',
  formats: 'Output formats',
  generate: 'Generate subset',
  generatedFiles: 'Generated files',
  localOnly: 'Local processing only — your font never leaves this browser.',
  processing: {
    archiving: 'Packaging your ZIP…',
    complete: 'Ready to download.',
    converting: 'Generating selected formats…',
    error: 'Generation failed.',
    idle: 'Choose a font and characters to begin.',
    initializing: 'Starting the font engine…',
    normalizing: 'Preparing the input font…',
    subsetting: 'Keeping requested characters…',
  },
  replaceFile: 'Replace font file',
  resultsHelp: 'Your generated font files will appear here.',
  selectFormats: 'Choose at least one font format. CSS needs a font source.',
  title: 'Font subset, in your browser',
  uniqueCodePoints: 'unique code points',
  unicodeRanges: 'Unicode ranges for CSS',
  unicodeRangesHelp:
    'Optional, comma-separated values such as U+0020-007E. These guide browser font matching; they do not change the subset.',
  uploadHelp:
    'TTF, WOFF, WOFF2, EOT, OTF, or SVG Font · drag and drop supported',
}

const chineseCopy: PlaygroundCopy = {
  characters: '要保留的字符',
  charactersHelp: '个字符',
  changeFile: '更换',
  chooseFile: '选择字体文件',
  coverage: '字符覆盖率',
  coverageComplete: '字体支持全部请求字符。',
  coverageMissing: '缺失码点',
  coverageRequested: '请求',
  coverageSupported: '支持',
  download: '下载 ZIP',
  downloadAsset: '下载文件',
  downloadZip: '下载 ZIP',
  delivery: 'Unicode 分片交付',
  deliveryCjk: 'CJK（U+4E00-9FFF）',
  deliveryCustom: '自定义分片',
  deliveryCustomHelp: '启用后必须填写自定义范围。',
  deliveryHelp:
    '可选：每个启用的分片都会生成独立字体文件与 CSS unicode-range 描述符。',
  deliveryLatin: 'Latin（U+0000-00FF）',
  dropFile: '拖入字体文件即可替换当前选择。',
  fontFile: '字体文件',
  formats: '输出格式',
  generate: '生成子集字体',
  generatedFiles: '已生成文件',
  localOnly: '仅在本地浏览器处理，字体文件不会上传。',
  processing: {
    archiving: '正在打包 ZIP…',
    complete: '已生成，可下载。',
    converting: '正在生成所选格式…',
    error: '生成失败。',
    idle: '选择字体并输入字符后开始。',
    initializing: '正在启动字体引擎…',
    normalizing: '正在准备输入字体…',
    subsetting: '正在保留所需字符…',
  },
  replaceFile: '替换字体文件',
  resultsHelp: '生成后的字体文件会显示在这里。',
  selectFormats: '至少选择一种字体格式；CSS 需要一个字体来源。',
  title: '在浏览器内提取字体子集',
  uniqueCodePoints: '个去重码点',
  unicodeRanges: 'CSS Unicode 范围',
  unicodeRangesHelp:
    '可选，使用逗号分隔，例如 U+0020-007E。这些范围决定浏览器的字体匹配，不会改变子集内容。',
  uploadHelp: 'TTF、WOFF、WOFF2、EOT、OTF 或 SVG Font · 支持拖放',
}

const copy = computed(() => (props.locale === 'zh' ? chineseCopy : englishCopy))
const playground = useFontPlayground()
const acceptedFonts = '.ttf,.woff,.woff2,.eot,.otf,.svg'
const { onChange, open } = useFileDialog({
  accept: acceptedFonts,
  multiple: false,
  reset: true,
})

onChange(files => {
  const file = files?.item(0)
  if (file) {
    playground.selectFile(file)
  }
})

const { isOverDropZone } = useDropZone(document, {
  multiple: false,
  preventDefaultForUnhandled: true,
  onDrop(files) {
    const file = files?.[0]
    if (file) {
      playground.selectFile(file)
    }
  },
})
</script>

<template>
  <section
    class="mx-auto my-8 w-full max-w-[var(--vp-layout-max-width)] px-5 sm:px-8 lg:px-12"
    aria-labelledby="playground-title"
  >
    <header class="mb-8 border-b border-[var(--vp-c-border)] pb-5">
      <p
        class="m-0 text-xs font-semibold tracking-[0.12em] text-[var(--vp-c-brand-1)] uppercase"
      >
        fontmin-rs / WASM
      </p>
      <h1
        id="playground-title"
        class="mb-0 mt-2 text-3xl font-semibold tracking-tight text-[var(--vp-c-text-1)] sm:text-4xl"
      >
        {{ copy.title }}
      </h1>
      <p
        class="mb-0 mt-3 max-w-2xl text-sm leading-6 text-[var(--vp-c-text-2)]"
      >
        {{ copy.localOnly }}
      </p>
    </header>

    <PlaygroundForm
      :can-generate="playground.canGenerate.value"
      :characters="playground.characters.value"
      :copy="copy"
      :custom-delivery-ranges="playground.customDeliveryRanges.value"
      :delivery-presets="playground.selectedDeliveryPresets.value"
      :file="playground.selectedFile.value"
      :formats="playground.selectedFormats.value"
      :unicode-ranges="playground.unicodeRanges.value"
      :unique-code-points="playground.uniqueCodePoints.value"
      @generate="playground.generate"
      @open-file-dialog="open"
      @update-characters="playground.setCharacters"
      @update-custom-delivery-ranges="playground.setCustomDeliveryRanges"
      @update-delivery-preset="playground.setDeliveryPreset"
      @update-format="playground.setFormat"
      @update-unicode-ranges="playground.setUnicodeRanges"
    />

    <PlaygroundResults
      :assets="playground.assets.value"
      :copy="copy"
      :coverage="playground.coverage.value"
      :error="playground.error.value"
      :phase="playground.phase.value"
      @download-archive="playground.download"
      @download-asset="playground.downloadAsset"
    />

    <div
      v-if="isOverDropZone"
      class="pointer-events-none fixed inset-0 z-50 grid place-items-center bg-[color-mix(in_srgb,var(--vp-c-bg)_80%,transparent)] p-6 backdrop-blur-sm"
      data-testid="drop-overlay"
    >
      <div
        class="flex items-center gap-3 rounded-xl border border-[var(--vp-c-brand-1)] bg-[var(--vp-c-bg)] px-5 py-4 text-sm font-semibold text-[var(--vp-c-text-1)] shadow-xl"
      >
        <span
          class="i-lucide-file-down size-5 text-[var(--vp-c-brand-1)]"
          aria-hidden="true"
        />
        {{ copy.dropFile }}
      </div>
    </div>
  </section>
</template>
