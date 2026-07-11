<script setup lang="ts">
import { computed } from 'vue'
import type {
  PlaygroundCopy,
  PlaygroundDeliveryPreset,
  PlaygroundFormat,
} from '../playground/types'

const props = defineProps<{
  canGenerate: boolean
  characters: string
  copy: PlaygroundCopy
  customDeliveryRanges: string
  deliveryPresets: ReadonlySet<PlaygroundDeliveryPreset>
  file?: File
  formats: ReadonlySet<PlaygroundFormat>
  unicodeRanges: string
  uniqueCodePoints: number
}>()

const emit = defineEmits<{
  generate: []
  openFileDialog: []
  updateCharacters: [value: string]
  updateCustomDeliveryRanges: [value: string]
  updateDeliveryPreset: [preset: PlaygroundDeliveryPreset, selected: boolean]
  updateFormat: [format: PlaygroundFormat, selected: boolean]
  updateUnicodeRanges: [value: string]
}>()

const formatOptions: PlaygroundFormat[] = [
  'woff2',
  'woff',
  'ttf',
  'eot',
  'svg',
  'css',
]
const deliveryPresetOptions: PlaygroundDeliveryPreset[] = [
  'latin',
  'cjk',
  'custom',
]

const fileSummary = computed(() => {
  if (!props.file) return props.copy.uploadHelp
  return `${props.file.name} · ${formatBytes(props.file.size)}`
})

function formatBytes(size: number): string {
  if (size < 1024) return `${size} B`
  if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <section
    class="overflow-hidden rounded-xl border border-[var(--vp-c-border)] bg-[var(--vp-c-bg-soft)]"
  >
    <div
      data-testid="playground-row"
      class="grid gap-2 border-b border-[var(--vp-c-border)] px-4 py-4 md:grid-cols-[9rem_minmax(0,1fr)] md:items-start"
    >
      <p
        class="pt-2 text-xs font-semibold tracking-wide text-[var(--vp-c-text-2)] uppercase"
      >
        {{ copy.fontFile }}
      </p>
      <div class="flex min-w-0 items-center justify-between gap-3">
        <button
          class="flex min-w-0 items-center gap-3 rounded-lg border border-[var(--vp-c-border)] bg-[var(--vp-c-bg)] px-3 py-2 text-left text-sm text-[var(--vp-c-text-1)] transition-colors hover:border-[var(--vp-c-brand-1)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--vp-c-brand-1)]"
          data-testid="open-file-dialog"
          type="button"
          @click="emit('openFileDialog')"
        >
          <span
            class="i-lucide-file-type-2 size-4 shrink-0 text-[var(--vp-c-brand-1)]"
            aria-hidden="true"
          />
          <span class="truncate">{{ fileSummary }}</span>
        </button>
        <span class="shrink-0 text-xs font-medium text-[var(--vp-c-brand-1)]">
          {{ file ? copy.changeFile : copy.chooseFile }}
        </span>
      </div>
    </div>

    <div
      data-testid="playground-row"
      class="grid gap-2 border-b border-[var(--vp-c-border)] px-4 py-4 md:grid-cols-[9rem_minmax(0,1fr)] md:items-start"
    >
      <label
        for="playground-characters"
        class="pt-2 text-xs font-semibold tracking-wide text-[var(--vp-c-text-2)] uppercase"
      >
        {{ copy.characters }}
      </label>
      <div class="grid gap-2">
        <textarea
          id="playground-characters"
          :value="characters"
          class="min-h-24 w-full resize-y rounded-lg border border-[var(--vp-c-border)] bg-[var(--vp-c-bg)] px-3 py-2 text-sm leading-6 text-[var(--vp-c-text-1)] outline-none transition-colors placeholder:text-[var(--vp-c-text-3)] focus:border-[var(--vp-c-brand-1)] focus:ring-2 focus:ring-[var(--vp-c-brand-soft)]"
          :placeholder="copy.charactersHelp"
          @input="
            emit(
              'updateCharacters',
              ($event.target as HTMLTextAreaElement).value,
            )
          "
        />
        <p class="text-xs text-[var(--vp-c-text-2)]">
          {{ characters.length }} {{ copy.charactersHelp }} ·
          {{ uniqueCodePoints }} {{ copy.uniqueCodePoints }}
        </p>
      </div>
    </div>

    <div
      data-testid="playground-row"
      class="grid gap-2 border-b border-[var(--vp-c-border)] px-4 py-4 md:grid-cols-[9rem_minmax(0,1fr)] md:items-start"
    >
      <p
        id="playground-formats"
        class="pt-2 text-xs font-semibold tracking-wide text-[var(--vp-c-text-2)] uppercase"
      >
        {{ copy.formats }}
      </p>
      <div class="grid gap-2">
        <div
          class="flex flex-wrap gap-2"
          role="group"
          aria-labelledby="playground-formats"
        >
          <label
            v-for="format in formatOptions"
            :key="format"
            class="inline-flex cursor-pointer items-center gap-2 rounded-md border border-[var(--vp-c-border)] bg-[var(--vp-c-bg)] px-2.5 py-1.5 text-xs font-medium text-[var(--vp-c-text-1)] transition-colors has-[:checked]:border-[var(--vp-c-brand-1)] has-[:checked]:bg-[var(--vp-c-brand-soft)]"
          >
            <input
              :checked="formats.has(format)"
              class="size-3 accent-[var(--vp-c-brand-1)]"
              type="checkbox"
              :value="format"
              @change="
                emit(
                  'updateFormat',
                  format,
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
            {{ format.toUpperCase() }}
          </label>
        </div>
        <p class="text-xs text-[var(--vp-c-text-2)]">
          {{ copy.selectFormats }}
        </p>
      </div>
    </div>

    <div
      v-if="formats.has('css')"
      data-testid="playground-row"
      class="grid gap-2 border-b border-[var(--vp-c-border)] px-4 py-4 md:grid-cols-[9rem_minmax(0,1fr)] md:items-start"
    >
      <label
        for="playground-unicode-ranges"
        class="pt-2 text-xs font-semibold tracking-wide text-[var(--vp-c-text-2)] uppercase"
      >
        {{ copy.unicodeRanges }}
      </label>
      <div class="grid gap-2">
        <input
          id="playground-unicode-ranges"
          data-testid="playground-unicode-ranges"
          :value="unicodeRanges"
          aria-describedby="playground-unicode-ranges-help"
          class="w-full rounded-lg border border-[var(--vp-c-border)] bg-[var(--vp-c-bg)] px-3 py-2 font-mono text-sm text-[var(--vp-c-text-1)] outline-none transition-colors placeholder:text-[var(--vp-c-text-3)] focus:border-[var(--vp-c-brand-1)] focus:ring-2 focus:ring-[var(--vp-c-brand-soft)]"
          placeholder="U+0020-007E, U+4E00-9FFF"
          type="text"
          @input="
            emit(
              'updateUnicodeRanges',
              ($event.target as HTMLInputElement).value,
            )
          "
        />
        <p
          id="playground-unicode-ranges-help"
          class="text-xs text-[var(--vp-c-text-2)]"
        >
          {{ copy.unicodeRangesHelp }}
        </p>
      </div>
    </div>

    <div
      v-if="formats.has('css')"
      data-testid="playground-row"
      class="grid gap-2 border-b border-[var(--vp-c-border)] px-4 py-4 md:grid-cols-[9rem_minmax(0,1fr)] md:items-start"
    >
      <p
        id="playground-delivery"
        class="pt-2 text-xs font-semibold tracking-wide text-[var(--vp-c-text-2)] uppercase"
      >
        {{ copy.delivery }}
      </p>
      <div class="grid gap-2">
        <div
          class="flex flex-wrap gap-2"
          role="group"
          aria-labelledby="playground-delivery"
        >
          <label
            v-for="preset in deliveryPresetOptions"
            :key="preset"
            class="inline-flex cursor-pointer items-center gap-2 rounded-md border border-[var(--vp-c-border)] bg-[var(--vp-c-bg)] px-2.5 py-1.5 text-xs font-medium text-[var(--vp-c-text-1)] transition-colors has-[:checked]:border-[var(--vp-c-brand-1)] has-[:checked]:bg-[var(--vp-c-brand-soft)]"
          >
            <input
              :checked="deliveryPresets.has(preset)"
              :data-testid="`playground-delivery-${preset}`"
              class="size-3 accent-[var(--vp-c-brand-1)]"
              type="checkbox"
              :value="preset"
              @change="
                emit(
                  'updateDeliveryPreset',
                  preset,
                  ($event.target as HTMLInputElement).checked,
                )
              "
            />
            {{
              preset === 'latin'
                ? copy.deliveryLatin
                : preset === 'cjk'
                  ? copy.deliveryCjk
                  : copy.deliveryCustom
            }}
          </label>
        </div>
        <input
          v-if="deliveryPresets.has('custom')"
          id="playground-delivery-custom-ranges"
          data-testid="playground-delivery-custom-ranges"
          :value="customDeliveryRanges"
          aria-describedby="playground-delivery-custom-help"
          class="w-full rounded-lg border border-[var(--vp-c-border)] bg-[var(--vp-c-bg)] px-3 py-2 font-mono text-sm text-[var(--vp-c-text-1)] outline-none transition-colors placeholder:text-[var(--vp-c-text-3)] focus:border-[var(--vp-c-brand-1)] focus:ring-2 focus:ring-[var(--vp-c-brand-soft)]"
          placeholder="U+0020-007E, U+4E00-9FFF"
          type="text"
          @input="
            emit(
              'updateCustomDeliveryRanges',
              ($event.target as HTMLInputElement).value,
            )
          "
        />
        <p
          id="playground-delivery-custom-help"
          class="text-xs text-[var(--vp-c-text-2)]"
        >
          {{ copy.deliveryHelp }} {{ copy.deliveryCustomHelp }}
        </p>
      </div>
    </div>

    <div
      data-testid="playground-row"
      class="flex flex-wrap items-center justify-between gap-3 px-4 py-4"
    >
      <p class="text-xs text-[var(--vp-c-text-2)]">
        {{ copy.processing.idle }}
      </p>
      <button
        class="inline-flex items-center gap-2 rounded-lg bg-[var(--vp-c-brand-1)] px-4 py-2.5 text-sm font-semibold text-white transition-colors hover:bg-[var(--vp-c-brand-2)] disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--vp-c-brand-1)]"
        data-testid="generate"
        :disabled="!canGenerate"
        type="button"
        @click="emit('generate')"
      >
        <span
          class="i-lucide-sparkles size-4"
          aria-hidden="true"
        />
        {{ copy.generate }}
      </button>
    </div>
  </section>
</template>
