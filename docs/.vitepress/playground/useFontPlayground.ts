import { computed, shallowRef } from 'vue'
import {
  downloadArchive,
  downloadAsset as downloadPlaygroundAsset,
} from './archive'
import {
  createDeliverySlices,
  isSupportedInputFile,
  parseUnicodeRanges,
  processFont,
} from './font'
import type {
  PlaygroundDeliveryPreset,
  PlaygroundAsset,
  PlaygroundFormat,
  PlaygroundPhase,
  ProcessFontRequest,
} from './types'

const defaultFormats: PlaygroundFormat[] = ['woff2', 'woff', 'css']

export function useFontPlayground() {
  const assets = shallowRef<PlaygroundAsset[]>([])
  const characters = shallowRef('')
  const customDeliveryRanges = shallowRef('')
  const error = shallowRef('')
  const isGenerating = shallowRef(false)
  const phase = shallowRef<PlaygroundPhase>('idle')
  const selectedFile = shallowRef<File>()
  const selectedDeliveryPresets = shallowRef<Set<PlaygroundDeliveryPreset>>(
    new Set(),
  )
  const selectedFormats = shallowRef(new Set<PlaygroundFormat>(defaultFormats))
  const unicodeRanges = shallowRef('')

  const canGenerate = computed(
    () =>
      selectedFile.value !== undefined &&
      characters.value.trim().length > 0 &&
      selectedFormats.value.size > 0 &&
      !(selectedFormats.value.size === 1 && selectedFormats.value.has('css')) &&
      !isGenerating.value,
  )
  const uniqueCodePoints = computed(() => new Set(characters.value).size)

  function selectFile(file: File): void {
    if (!isSupportedInputFile(file.name)) {
      const extension = file.name.split('.').pop() || 'unknown'
      error.value = `Unsupported input format: .${extension}.`
      return
    }
    selectedFile.value = file
    error.value = ''
  }

  function setCharacters(value: string): void {
    characters.value = value
  }

  function setCustomDeliveryRanges(value: string): void {
    customDeliveryRanges.value = value
  }

  function setDeliveryPreset(
    preset: PlaygroundDeliveryPreset,
    selected: boolean,
  ): void {
    const next = new Set(selectedDeliveryPresets.value)
    if (selected) {
      next.add(preset)
    } else {
      next.delete(preset)
    }
    selectedDeliveryPresets.value = next
  }

  function setFormat(format: PlaygroundFormat, selected: boolean): void {
    const next = new Set(selectedFormats.value)
    if (selected) {
      next.add(format)
    } else {
      next.delete(format)
    }
    selectedFormats.value = next
  }

  function setUnicodeRanges(value: string): void {
    unicodeRanges.value = value
  }

  async function generate(): Promise<void> {
    const file = selectedFile.value
    if (!file || !canGenerate.value) {
      return
    }

    isGenerating.value = true
    error.value = ''

    try {
      const request: ProcessFontRequest = {
        contents: new Uint8Array(await file.arrayBuffer()),
        fileName: file.name,
        formats: selectedFormats.value,
        onPhase: nextPhase => {
          phase.value = nextPhase
        },
        text: characters.value,
      }

      if (selectedFormats.value.has('css')) {
        request.unicodeRanges = parseUnicodeRanges(unicodeRanges.value)
        const deliverySlices = createDeliverySlices(
          selectedDeliveryPresets.value,
          customDeliveryRanges.value,
        )

        if (deliverySlices.length > 0) {
          request.deliverySlices = deliverySlices
        }
      }

      assets.value = await processFont(request)
      phase.value = 'complete'
    } catch (caught) {
      error.value = caught instanceof Error ? caught.message : String(caught)
      phase.value = 'error'
    } finally {
      isGenerating.value = false
    }
  }

  function download(): void {
    const file = selectedFile.value
    if (!file || assets.value.length === 0) {
      return
    }

    phase.value = 'archiving'
    downloadArchive(assets.value, `${fileStem(file.name)}-fontmin.zip`)
    phase.value = 'complete'
  }

  function downloadAsset(asset: PlaygroundAsset): void {
    downloadPlaygroundAsset(asset)
  }

  return {
    assets,
    canGenerate,
    characters,
    customDeliveryRanges,
    download,
    downloadAsset,
    error,
    generate,
    phase,
    selectFile,
    selectedFile,
    selectedDeliveryPresets,
    selectedFormats,
    setCharacters,
    setCustomDeliveryRanges,
    setDeliveryPreset,
    setFormat,
    setUnicodeRanges,
    unicodeRanges,
    uniqueCodePoints,
  }
}

function fileStem(fileName: string): string {
  return fileName.replace(/\.[^.]+$/u, '') || 'fontmin'
}
