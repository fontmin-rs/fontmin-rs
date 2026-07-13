<script setup lang="ts">
import type {
  PlaygroundAsset,
  PlaygroundCopy,
  PlaygroundPhase,
} from '../playground/types'

defineProps<{
  assets: PlaygroundAsset[]
  copy: PlaygroundCopy
  error: string
  phase: PlaygroundPhase
}>()

const emit = defineEmits<{
  downloadArchive: []
  downloadAsset: [asset: PlaygroundAsset]
}>()

function formatBytes(size: number): string {
  if (size < 1024) {
    return `${size} B`
  }
  if (size < 1024 * 1024) {
    return `${(size / 1024).toFixed(1)} KB`
  }
  return `${(size / (1024 * 1024)).toFixed(1)} MB`
}
</script>

<template>
  <section class="mt-10">
    <div
      class="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--vp-c-border)] pb-3"
    >
      <div>
        <h2 class="m-0 text-base font-semibold text-[var(--vp-c-text-1)]">
          {{ copy.generatedFiles }}
        </h2>
        <p
          class="mt-1 text-sm text-[var(--vp-c-text-2)]"
          role="status"
        >
          {{ copy.processing[phase] }}
        </p>
      </div>
      <button
        v-if="assets.length > 0"
        class="inline-flex items-center gap-2 rounded-lg border border-[var(--vp-c-border)] bg-[var(--vp-c-bg-soft)] px-3 py-2 text-sm font-semibold text-[var(--vp-c-text-1)] transition-colors hover:border-[var(--vp-c-brand-1)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--vp-c-brand-1)]"
        data-testid="download-archive"
        type="button"
        @click="emit('downloadArchive')"
      >
        <span
          class="i-lucide-package-down size-4 text-[var(--vp-c-brand-1)]"
          aria-hidden="true"
        />
        {{ copy.downloadZip }}
      </button>
    </div>

    <p
      v-if="error"
      class="mt-4 text-sm text-[var(--vp-c-danger-1)]"
      role="alert"
    >
      {{ error }}
    </p>
    <p
      v-else-if="assets.length === 0"
      class="mt-4 text-sm text-[var(--vp-c-text-2)]"
    >
      {{ copy.resultsHelp }}
    </p>
    <ul
      v-else
      class="m-0 list-none divide-y divide-[var(--vp-c-border)] p-0"
    >
      <li
        v-for="asset in assets"
        :key="asset.fileName"
        class="grid grid-cols-[auto_minmax(0,1fr)_auto_auto] items-center gap-3 py-3 text-sm"
      >
        <span
          class="rounded bg-[var(--vp-c-brand-soft)] px-1.5 py-0.5 text-[0.65rem] font-bold tracking-wide text-[var(--vp-c-brand-1)]"
          >{{ asset.format.toUpperCase() }}</span
        >
        <span class="truncate text-[var(--vp-c-text-1)]">{{
          asset.fileName
        }}</span>
        <span
          class="font-mono text-xs tabular-nums text-[var(--vp-c-text-2)]"
          >{{ formatBytes(asset.contents.byteLength) }}</span
        >
        <button
          :aria-label="`${copy.downloadAsset}: ${asset.fileName}`"
          :data-testid="`download-asset-${asset.fileName}`"
          class="grid size-8 place-items-center rounded-md text-[var(--vp-c-brand-1)] transition-colors hover:bg-[var(--vp-c-default-soft)] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--vp-c-brand-1)]"
          type="button"
          @click="emit('downloadAsset', asset)"
        >
          <span
            class="i-lucide-download size-4"
            aria-hidden="true"
          />
        </button>
      </li>
    </ul>
  </section>
</template>
