import { mount } from '@vue/test-utils'
import { describe, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import { downloadArchive, downloadAsset } from '../playground/archive'
import { processFont } from '../playground/font'
import type { PlaygroundAsset, ProcessFontRequest } from '../playground/types'
import FontPlayground from './FontPlayground.vue'

type FileDialogChange = (callback: (files: FileList | null) => void) => void
type DropRegistration = (callback: (files: File[] | null) => void) => void

const { fileDialogOnChange, openFileDialog, registerDrop } = vi.hoisted(() => ({
  fileDialogOnChange: vi.fn<FileDialogChange>(),
  openFileDialog: vi.fn<() => void>(),
  registerDrop: vi.fn<DropRegistration>(),
}))

vi.mock(import('../playground/archive'), () => ({
  downloadArchive:
    vi.fn<(assets: PlaygroundAsset[], fileName: string) => void>(),
  downloadAsset: vi.fn<(asset: PlaygroundAsset) => void>(),
}))
vi.mock(import('../playground/font'), () => ({
  createDeliverySlices: (
    presets: ReadonlySet<'latin' | 'cjk' | 'custom'>,
    customRanges: string,
  ) => {
    if (presets.has('custom') && customRanges.includes('?')) {
      throw new Error(`Invalid Unicode range: ${customRanges}.`)
    }

    return [...presets].map(preset => {
      let unicodeRanges: string[]
      if (preset === 'latin') {
        unicodeRanges = ['U+0000-00FF']
      } else if (preset === 'cjk') {
        unicodeRanges = ['U+4E00-9FFF']
      } else {
        unicodeRanges = customRanges.split(',').map(range => range.trim())
      }

      return { name: preset, unicodeRanges }
    })
  },
  isSupportedInputFile: vi.fn<(fileName: string) => boolean>(() => true),
  parseUnicodeRanges: (value: string) => {
    if (value.includes('?')) {
      throw new Error(`Invalid Unicode range: ${value}.`)
    }

    return value.length === 0 ? [] : value.split(',').map(range => range.trim())
  },
  processFont:
    vi.fn<(request: ProcessFontRequest) => Promise<PlaygroundAsset[]>>(),
}))

vi.mock(import('@vueuse/core'), async () => {
  const { shallowRef } = await import('vue')

  return {
    useDropZone: (_target: Document, options: { onDrop: unknown }) => {
      registerDrop(options.onDrop)
      return { isOverDropZone: shallowRef(false) }
    },
    useFileDialog: () => ({
      files: shallowRef(null),
      onChange: fileDialogOnChange,
      open: openFileDialog,
      reset: () => {},
    }),
  }
})

async function selectFile(): Promise<void> {
  const file = new File([new Uint8Array([1])], 'demo.ttf')
  const files = {
    0: file,
    item: (index: number) => (index === 0 ? file : null),
    length: 1,
  }

  const callback = fileDialogOnChange.mock.calls.at(-1)?.[0] as
    | ((value: FileList | null) => void)
    | undefined
  if (!callback) {
    throw new Error('The file dialog was not registered.')
  }

  callback(files as unknown as FileList)
  await nextTick()
}

describe('FontPlayground', () => {
  it('disables generation until a font file and character set are supplied', async () => {
    const wrapper = mount(FontPlayground)

    expect(
      wrapper.get('[data-testid="generate"]').attributes('disabled'),
    ).toBeDefined()

    await wrapper.get('textarea').setValue('Hello')
    expect(
      wrapper.get('[data-testid="generate"]').attributes('disabled'),
    ).toBeDefined()

    await selectFile()
    expect(
      wrapper.get('[data-testid="generate"]').attributes('disabled'),
    ).toBeUndefined()
  })

  it('lists generated assets and downloads their ZIP on demand', async () => {
    vi.mocked(processFont).mockResolvedValue([
      {
        contents: new Uint8Array([1]),
        fileName: 'demo.woff2',
        format: 'woff2',
      },
      {
        contents: new TextEncoder().encode('@font-face {}'),
        fileName: 'demo.css',
        format: 'css',
      },
    ])
    const wrapper = mount(FontPlayground)

    await selectFile()
    await wrapper.get('textarea').setValue('Hello')
    await wrapper.get('[data-testid="generate"]').trigger('click')

    expect(wrapper.text()).toContain('demo.woff2')
    expect(wrapper.findAll('[data-testid="playground-row"]')).toHaveLength(6)
    expect(
      wrapper
        .get('[data-testid="download-asset-demo.woff2"]')
        .attributes('aria-label'),
    ).toContain('demo.woff2')
    expect(wrapper.get('[data-testid="download-archive"]').exists()).toBe(true)

    await wrapper
      .get('[data-testid="download-asset-demo.woff2"]')
      .trigger('click')
    expect(downloadAsset).toHaveBeenCalledWith(
      expect.objectContaining({ fileName: 'demo.woff2' }),
    )

    await wrapper.get('[data-testid="download-archive"]').trigger('click')
    expect(downloadArchive).toHaveBeenCalledWith(
      expect.any(Array),
      'demo-fontmin.zip',
    )
  })

  it('shows requested character coverage and missing code points', async () => {
    vi.mocked(processFont).mockImplementation(async request => {
      request.onCoverage?.({
        coveragePercent: 50,
        missing: [134_071],
        requested: [0x41, 134_071],
        supported: [0x41],
      })

      return []
    })
    const wrapper = mount(FontPlayground)

    await selectFile()
    await wrapper.get('textarea').setValue('A𠮷')
    await wrapper.get('[data-testid="generate"]').trigger('click')

    expect(wrapper.get('[data-testid="coverage-report"]').text()).toContain(
      '50%',
    )
    expect(wrapper.get('[data-testid="coverage-missing"]').text()).toContain(
      'U+20BB7',
    )
  })

  it('forwards Unicode ranges only when CSS output is selected', async () => {
    vi.mocked(processFont).mockResolvedValue([])
    const wrapper = mount(FontPlayground)

    await selectFile()
    await wrapper.get('textarea').setValue('Hello')
    await wrapper
      .get('[data-testid="playground-unicode-ranges"]')
      .setValue('U+0020-007E, U+4E00-9FFF')
    await wrapper.get('[data-testid="generate"]').trigger('click')

    expect(processFont).toHaveBeenLastCalledWith(
      expect.objectContaining({
        unicodeRanges: ['U+0020-007E', 'U+4E00-9FFF'],
      }),
    )

    await wrapper.get('input[value="css"]').setValue(false)
    expect(
      wrapper.find('[data-testid="playground-unicode-ranges"]').exists(),
    ).toBe(false)
  })

  it('forwards enabled delivery presets and custom ranges', async () => {
    vi.mocked(processFont).mockResolvedValue([])
    const wrapper = mount(FontPlayground)

    await selectFile()
    await wrapper.get('textarea').setValue('Hello')
    await wrapper
      .get('[data-testid="playground-delivery-latin"]')
      .setValue(true)
    await wrapper
      .get('[data-testid="playground-delivery-custom"]')
      .setValue(true)
    await wrapper
      .get('[data-testid="playground-delivery-custom-ranges"]')
      .setValue('U+0041-004D, U+004E-005A')
    await wrapper.get('[data-testid="generate"]').trigger('click')

    expect(processFont).toHaveBeenLastCalledWith(
      expect.objectContaining({
        deliverySlices: [
          { name: 'latin', unicodeRanges: ['U+0000-00FF'] },
          {
            name: 'custom',
            unicodeRanges: ['U+0041-004D', 'U+004E-005A'],
          },
        ],
      }),
    )
  })

  it('shows malformed custom delivery ranges before processing a font', async () => {
    const wrapper = mount(FontPlayground)

    await selectFile()
    await wrapper.get('textarea').setValue('Hello')
    await wrapper
      .get('[data-testid="playground-delivery-custom"]')
      .setValue(true)
    await wrapper
      .get('[data-testid="playground-delivery-custom-ranges"]')
      .setValue('U+4??')
    await wrapper.get('[data-testid="generate"]').trigger('click')

    expect(wrapper.get('[role="alert"]').text()).toContain(
      'Invalid Unicode range: U+4??.',
    )
  })

  it('shows the malformed Unicode range before processing a font', async () => {
    const wrapper = mount(FontPlayground)

    await selectFile()
    await wrapper.get('textarea').setValue('Hello')
    await wrapper
      .get('[data-testid="playground-unicode-ranges"]')
      .setValue('U+4??')
    await wrapper.get('[data-testid="generate"]').trigger('click')

    expect(wrapper.get('[role="alert"]').text()).toContain(
      'Invalid Unicode range: U+4??.',
    )
  })

  it('keeps the user-facing failure message when generation fails', async () => {
    vi.mocked(processFont).mockRejectedValue(new Error('Invalid font data'))
    const wrapper = mount(FontPlayground)

    await selectFile()
    await wrapper.get('textarea').setValue('Hello')
    await wrapper.get('[data-testid="generate"]').trigger('click')

    expect(wrapper.get('[role="alert"]').text()).toContain('Invalid font data')
  })

  it('opens the native font picker from the file row', async () => {
    const wrapper = mount(FontPlayground)

    await wrapper.get('[data-testid="open-file-dialog"]').trigger('click')

    expect(openFileDialog).toHaveBeenCalledOnce()
  })

  it('selects a font dropped over the page', async () => {
    const wrapper = mount(FontPlayground)
    const drop = registerDrop.mock.calls.at(-1)?.[0] as
      | ((files: File[] | null) => void)
      | undefined

    if (!drop) {
      throw new Error('The drop handler was not registered.')
    }
    drop([new File([new Uint8Array([1])], 'dropped.woff2')])
    await nextTick()

    expect(wrapper.text()).toContain('dropped.woff2')
  })
})
