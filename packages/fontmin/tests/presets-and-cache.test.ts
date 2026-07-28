import { execFileSync } from 'node:child_process'
import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { expect, it, vi } from 'vitest'
import {
  css,
  definePlugin,
  glyph,
  fontminCompatPreset,
  inspect,
  loadConfig,
  modernWeb,
  optimize,
  ttf2woff,
  ttf2woff2,
} from '../src/index'
import { fontminCompatPreset as fontminCompatPresetFromSubpath } from '../src/presets'
import {
  currentDir,
  fixture,
  cffFixture,
  cff2Fixture,
  bin,
} from './api-fixtures'

it('optimizes a modern web font preset', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-modern-web-'))

  try {
    const input = readFileSync(fixture)
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      plugins: modernWeb({
        text: 'Hello',
        fontFamily: 'Roboto',
        fontPath: './',
      }),
    })
    const paths = files.map(file => file.path).sort()
    const ttf = files.find(file => file.path === 'roboto-regular.ttf')
    const woff = files.find(file => file.path === 'roboto-regular.woff')
    const woff2 = files.find(file => file.path === 'roboto-regular.woff2')
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(paths).toStrictEqual([
      'roboto-regular.css',
      'roboto-regular.ttf',
      'roboto-regular.woff',
      'roboto-regular.woff2',
    ])
    expect(ttf).toBeDefined()
    expect(woff).toBeDefined()
    expect(woff2).toBeDefined()
    expect(cssAsset).toBeDefined()
    if (
      ttf === undefined ||
      woff === undefined ||
      woff2 === undefined ||
      cssAsset === undefined
    ) {
      throw new Error('modernWeb did not emit expected assets')
    }
    expect(ttf.contents.byteLength).toBeLessThan(input.byteLength)
    expect(Buffer.from(woff.contents.subarray(0, 4)).toString('ascii')).toBe(
      'wOFF',
    )
    expect(Buffer.from(woff2.contents.subarray(0, 4)).toString('ascii')).toBe(
      'wOF2',
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "font-family: 'Roboto';",
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('keeps modern web options scoped to their built-in descriptors', () => {
  const plugins = modernWeb({
    text: 'Hello',
    fontFamily: 'Roboto',
    fontPath: './fonts',
    compressionLevel: 6,
    quality: 9,
  })

  expect(
    plugins.find(plugin => plugin.name === 'fontmin:ttf2woff')?.native,
  ).toStrictEqual({
    kind: 'builtin',
    name: 'ttf2woff',
    options: { compressionLevel: 6 },
  })
  expect(
    plugins.find(plugin => plugin.name === 'fontmin:ttf2woff2')?.native,
  ).toStrictEqual({
    kind: 'builtin',
    name: 'ttf2woff2',
    options: { quality: 9 },
  })
})

it('applies fileName and ext overrides to configured TTF outputs', async () => {
  const files = await optimize({
    input: [fixture],
    outputs: [
      { format: 'ttf', fileName: 'nested/project-font.bin' },
      { format: 'woff' },
    ],
  })

  expect(files.map(file => file.path).sort()).toStrictEqual([
    'nested/project-font.bin',
    'roboto-regular.woff',
  ])
})

it('rejects output traversal and refuses to clean the project root', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-safe-output-'))
  const sentinel = resolve(workDir, 'sentinel.txt')
  const inputDirectory = resolve(workDir, 'inputs')
  const inputPath = resolve(inputDirectory, 'font.ttf')

  writeFileSync(sentinel, 'keep')
  mkdirSync(inputDirectory)
  writeFileSync(inputPath, readFileSync(fixture))

  try {
    await expect(
      optimize({
        cwd: workDir,
        input: [fixture],
        outDir: '.',
        clean: true,
        outputs: [{ format: 'ttf' }],
      }),
    ).rejects.toThrow('refusing to clean output directory')
    expect(readFileSync(sentinel, 'utf8')).toBe('keep')

    await expect(
      optimize({
        cwd: workDir,
        input: [inputPath],
        outDir: inputDirectory,
        clean: true,
        outputs: [{ format: 'ttf' }],
      }),
    ).rejects.toThrow('refusing to clean output directory')
    expect(existsSync(inputPath)).toBe(true)

    await expect(
      optimize({
        cwd: workDir,
        input: [fixture],
        outDir: 'dist',
        outputs: [{ format: 'ttf', fileName: '../escaped.ttf' }],
      }),
    ).rejects.toThrow('must stay within its destination directory')
    expect(existsSync(resolve(workDir, 'escaped.ttf'))).toBe(false)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('runs plugin buildEnd hooks after a transform failure', async () => {
  const events: string[] = []

  await expect(
    optimize({
      input: [fixture],
      plugins: [
        definePlugin({
          name: 'failing-lifecycle',
          buildStart() {
            events.push('start')
          },
          transform() {
            events.push('transform')
            throw new Error('intentional transform failure')
          },
          buildEnd() {
            events.push('end')
          },
        }),
      ],
    }),
  ).rejects.toThrow('intentional transform failure')
  expect(events).toStrictEqual(['start', 'transform', 'end'])
})

it('keeps the packaged CLI help, doctor, OTF, and option behavior aligned', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-parity-'))
  const output = resolve(workDir, 'converted.ttf')

  try {
    expect(
      execFileSync(process.execPath, [bin, '--help'], { encoding: 'utf8' }),
    ).toContain('Usage:')
    expect(
      execFileSync(process.execPath, [bin, 'doctor'], { encoding: 'utf8' }),
    ).toContain('doctor ok')

    execFileSync(process.execPath, [
      bin,
      'convert',
      cffFixture,
      '--format',
      'ttf',
      '--output',
      output,
    ])
    expect(readFileSync(output).subarray(0, 4)).toStrictEqual(
      Buffer.from([0, 1, 0, 0]),
    )
    expect(() =>
      execFileSync(process.execPath, [bin, 'inspect', fixture, '--unknown']),
    ).toThrow('Command failed')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('initializes WASM lazily before running custom hooks', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-wasm-optimize-'))
  const transforms: string[] = []

  try {
    vi.resetModules()
    const { isWasmInitialized } = await import('@fontmin-rs/wasm')
    const { optimize: optimizeWithFreshRuntime } =
      await import('../src/optimize')

    expect(isWasmInitialized()).toBe(false)

    const files = await optimizeWithFreshRuntime({
      input: [fixture],
      outDir: outputDir,
      runtime: 'wasm',
      plugins: [
        definePlugin({
          name: 'wasm-custom-hook',
          transform(asset, context) {
            transforms.push(asset.path)
            context.emitFile({
              path: 'custom-hook.txt',
              contents: Buffer.from('custom hook ran'),
              format: 'unknown',
              sourceFormat: asset.sourceFormat,
              meta: { plugin: 'wasm-custom-hook' },
            })

            return asset
          },
        }),
        ...modernWeb({
          fontFamily: 'Roboto WASM',
          fontPath: './',
          text: 'Hello',
        }),
      ],
    })

    expect(isWasmInitialized()).toBe(true)

    const woff = files.find(file => file.format === 'woff')
    const woff2 = files.find(file => file.format === 'woff2')
    const cssAsset = files.find(file => file.format === 'css')
    const customAsset = files.find(file => file.path === 'custom-hook.txt')

    expect(transforms).toStrictEqual(['roboto-regular.ttf'])
    expect(new TextDecoder().decode(customAsset?.contents)).toBe(
      'custom hook ran',
    )
    expect(
      Buffer.from(woff?.contents ?? [])
        .subarray(0, 4)
        .toString('ascii'),
    ).toBe('wOFF')
    expect(
      Buffer.from(woff2?.contents ?? [])
        .subarray(0, 4)
        .toString('ascii'),
    ).toBe('wOF2')
    expect(new TextDecoder().decode(cssAsset?.contents)).toContain(
      "font-family: 'Roboto WASM';",
    )
    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
        .subarray(0, 4)
        .toString(),
    ).toBe('wOF2')
  } finally {
    rmSync(outputDir, { force: true, recursive: true })
  }
})

it('separates runtime-specific cache manifests', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-runtime-cache-'))
  const cacheDir = resolve(workDir, 'cache')

  try {
    await optimize({
      cache: { dir: cacheDir, enabled: true },
      input: [fixture],
      runtime: 'native',
      plugins: modernWeb({ text: 'Hello' }),
    })
    await optimize({
      cache: { dir: cacheDir, enabled: true },
      input: [fixture],
      runtime: 'wasm',
      plugins: modernWeb({ text: 'Hello' }),
    })
    const index = JSON.parse(
      readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'),
    ) as { entries: Record<string, unknown> }

    expect(Object.keys(index.entries)).toHaveLength(2)

    const manifests = Object.keys(index.entries).map(key =>
      JSON.parse(
        readFileSync(
          resolve(
            cacheDir,
            'v1',
            key.slice(0, 2),
            key.slice(2, 4),
            key,
            'index.json',
          ),
          'utf8',
        ),
      ),
    ) as { runtime: { requested: string; resolved: string | null } }[]

    expect(
      manifests
        .map(manifest => manifest.runtime)
        .sort(
          (left, right) =>
            left.resolved?.localeCompare(right.resolved ?? '') ?? 0,
        ),
    ).toStrictEqual([
      { requested: 'native', resolved: 'native' },
      { requested: 'wasm', resolved: 'wasm' },
    ])
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('rejects a cached manifest with a mismatched runtime identity', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-runtime-cache-'))
  const cacheDir = resolve(workDir, 'cache')
  const config = {
    cache: { dir: cacheDir, enabled: true },
    input: [fixture],
    runtime: 'native' as const,
    plugins: [ttf2woff({ clone: false })],
  }

  try {
    await optimize(config)
    const index = JSON.parse(
      readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'),
    ) as { entries: Record<string, unknown> }
    const [key] = Object.keys(index.entries)

    if (key === undefined) {
      throw new Error('runtime cache test did not write an index entry')
    }

    const manifestPath = resolve(
      cacheDir,
      'v1',
      key.slice(0, 2),
      key.slice(2, 4),
      key,
      'index.json',
    )
    for (const field of ['requested', 'resolved'] as const) {
      const manifest = JSON.parse(readFileSync(manifestPath, 'utf8')) as {
        runtime: { requested: string; resolved: string | null }
      }

      manifest.runtime[field] = 'wasm'
      writeFileSync(manifestPath, `${JSON.stringify(manifest, undefined, 2)}\n`)

      const files = await optimize(config)
      const rewritten = JSON.parse(readFileSync(manifestPath, 'utf8')) as {
        runtime: { requested: string; resolved: string | null }
      }

      expect(files[0]?.meta['cache']).toBeUndefined()
      expect(rewritten.runtime).toStrictEqual({
        requested: 'native',
        resolved: 'native',
      })
    }
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('uses the legacy WOFF2 wasm fallback when runtime is omitted', async () => {
  vi.resetModules()
  const { isWasmInitialized } = await import('@fontmin-rs/wasm')
  const { optimize: optimizeWithFreshRuntime } = await import('../src/optimize')

  expect(isWasmInitialized()).toBe(false)

  const files = await optimizeWithFreshRuntime({
    input: [fixture],
    plugins: [ttf2woff2({ fallback: 'wasm' })],
  })
  const woff2 = files.find(file => file.format === 'woff2')

  expect(isWasmInitialized()).toBe(true)
  expect(
    Buffer.from(woff2?.contents ?? [])
      .subarray(0, 4)
      .toString('ascii'),
  ).toBe('wOF2')
})

it('rejects a runtime that conflicts with WOFF2 fallback', async () => {
  await expect(
    optimize({
      input: [fixture],
      runtime: 'native',
      plugins: [ttf2woff2({ fallback: 'wasm' })],
    }),
  ).rejects.toThrow('runtime `native` conflicts with WOFF2 fallback `wasm`')
})

it('normalizes static CFF OTF input through the modern web preset', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-modern-cff-'))

  try {
    const files = await optimize({
      input: [cffFixture],
      outDir: outputDir,
      plugins: modernWeb({ text: 'Hello', fontFamily: 'Source Sans 3' }),
    })

    expect(files.map(file => file.path).sort()).toStrictEqual([
      'source-sans-3-regular.css',
      'source-sans-3-regular.ttf',
      'source-sans-3-regular.woff',
      'source-sans-3-regular.woff2',
    ])
    const ttf = files.find(file => file.path.endsWith('.ttf'))
    const woff2 = files.find(file => file.path.endsWith('.woff2'))

    expect(ttf).toBeDefined()
    expect(woff2).toBeDefined()
    if (ttf === undefined || woff2 === undefined) {
      throw new Error('modernWeb did not normalize static CFF input')
    }
    expect(inspect(ttf.contents).metadata.tables).toContain('glyf')
    expect(inspect(ttf.contents).metadata.tables).not.toContain('CFF ')
    expect(Buffer.from(woff2.contents.subarray(0, 4)).toString('ascii')).toBe(
      'wOF2',
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('instantiates CFF2 coordinates through the modern web preset', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-modern-cff2-'))

  try {
    const files = await optimize({
      input: [cff2Fixture],
      outDir: outputDir,
      plugins: modernWeb({
        text: 'Hello',
        variationCoordinates: { wght: 700, opsz: 14 },
      }),
    })
    const ttf = files.find(file => file.path.endsWith('.ttf'))

    expect(ttf).toBeDefined()
    if (ttf === undefined) {
      throw new Error('modernWeb did not normalize CFF2 input')
    }
    const info = inspect(ttf.contents)
    expect(info.metadata.tables).toContain('glyf')
    expect(info.metadata.tables).not.toContain('CFF2')
    expect(info.metadata.tables).not.toContain('fvar')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('optimizes a fontmin-compatible preset', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-compat-preset-'))

  try {
    const input = readFileSync(fixture)
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      plugins: fontminCompatPreset({
        cssGlyph: true,
        deflateWoff: true,
        fontFamily: 'Roboto Compat',
        fontPath: './',
        text: 'Hello',
      }),
    })
    const subpathPlugins = fontminCompatPresetFromSubpath({ text: 'Hello' })
    const paths = files.map(file => file.path).sort()
    const ttf = files.find(file => file.path === 'roboto-regular.ttf')
    const eot = files.find(file => file.path === 'roboto-regular.eot')
    const svg = files.find(file => file.path === 'roboto-regular.svg')
    const woff = files.find(file => file.path === 'roboto-regular.woff')
    const woff2 = files.find(file => file.path === 'roboto-regular.woff2')
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(subpathPlugins.map(plugin => plugin.name)).toStrictEqual([
      'fontmin:otf2ttf',
      'fontmin:glyph',
      'fontmin:ttf2eot',
      'fontmin:ttf2svg',
      'fontmin:ttf2woff',
      'fontmin:ttf2woff2',
      'fontmin:css',
    ])
    expect(paths).toStrictEqual([
      'roboto-regular.css',
      'roboto-regular.eot',
      'roboto-regular.svg',
      'roboto-regular.ttf',
      'roboto-regular.woff',
      'roboto-regular.woff2',
    ])
    expect(ttf).toBeDefined()
    expect(eot).toBeDefined()
    expect(svg).toBeDefined()
    expect(woff).toBeDefined()
    expect(woff2).toBeDefined()
    expect(cssAsset).toBeDefined()
    if (
      ttf === undefined ||
      eot === undefined ||
      svg === undefined ||
      woff === undefined ||
      woff2 === undefined ||
      cssAsset === undefined
    ) {
      throw new Error('fontminCompatPreset did not emit expected assets')
    }

    expect(ttf.contents.byteLength).toBeLessThan(input.byteLength)
    expect(Buffer.from(eot.contents).readUInt32LE(0)).toBe(
      eot.contents.byteLength,
    )
    expect(new TextDecoder().decode(svg.contents)).toContain(
      'font-family="Roboto',
    )
    expect(Buffer.from(woff.contents.subarray(0, 4)).toString('ascii')).toBe(
      'wOFF',
    )
    expect(Buffer.from(woff2.contents.subarray(0, 4)).toString('ascii')).toBe(
      'wOF2',
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "font-family: 'Roboto Compat';",
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('keeps fontmin-compatible preset options scoped to built-in descriptors', () => {
  const plugins = fontminCompatPreset({
    compressionLevel: 6,
    cssGlyph: true,
    deflate: true,
    fallback: 'auto',
    fontFamily: 'Roboto Compat',
    preserveHinting: true,
    quality: 9,
    text: 'Hello',
    variationCoordinates: { wght: 700 },
    version: 0x0002_0002,
  })

  expect(JSON.parse(JSON.stringify(plugins))).toStrictEqual([
    {
      name: 'fontmin:otf2ttf',
      native: {
        kind: 'builtin',
        name: 'otf2ttf',
        options: {
          preserveHinting: true,
          variationCoordinates: { wght: 700 },
        },
      },
    },
    {
      name: 'fontmin:glyph',
      native: {
        kind: 'builtin',
        name: 'glyph',
        options: { preserveHinting: true, text: 'Hello' },
      },
    },
    {
      name: 'fontmin:ttf2eot',
      native: {
        kind: 'builtin',
        name: 'ttf2eot',
        options: { version: 0x0002_0002 },
      },
    },
    {
      name: 'fontmin:ttf2svg',
      native: {
        kind: 'builtin',
        name: 'ttf2svg',
        options: { fontFamily: 'Roboto Compat' },
      },
    },
    {
      name: 'fontmin:ttf2woff',
      native: {
        kind: 'builtin',
        name: 'ttf2woff',
        options: { compressionLevel: 6, deflate: true },
      },
    },
    {
      name: 'fontmin:ttf2woff2',
      native: {
        kind: 'builtin',
        name: 'ttf2woff2',
        options: { fallback: 'auto', quality: 9 },
      },
    },
    {
      name: 'fontmin:css',
      native: {
        kind: 'builtin',
        name: 'css',
        options: { fontFamily: 'Roboto Compat', glyph: true },
      },
    },
  ])
})

it('inlines font assets and preserves Unicode ranges in Base64 CSS', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-base64-css-'))

  try {
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      plugins: [
        glyph({ text: 'Hello' }),
        ttf2woff({ clone: false }),
        css({
          base64: true,
          fontFamily: 'Roboto',
          unicodeRanges: ['U+0041-005A'],
        }),
      ],
    })
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(cssAsset).toBeDefined()
    if (cssAsset === undefined) {
      throw new Error('base64 CSS was not emitted')
    }

    const cssText = new TextDecoder().decode(cssAsset.contents)

    expect(cssText).toContain("url('data:font/woff;base64,")
    expect(cssText).toContain('unicode-range: U+0041-005A;')
    expect(cssText).not.toContain('roboto-regular.woff')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('keeps native and WASM optimize cache entries separate', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-runtime-cache-'))
  const cacheDir = resolve(workDir, 'cache')

  try {
    for (const runtime of ['native', 'wasm'] as const) {
      await optimize({
        input: [fixture],
        cache: { enabled: true, dir: cacheDir },
        runtime,
        plugins: modernWeb({ fontFamily: 'Roboto', text: 'Hello' }),
      })
    }

    const index = JSON.parse(
      readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'),
    ) as { entries: Record<string, unknown> }
    const keys = Object.keys(index.entries)
    const manifests = keys.map(key => {
      return JSON.parse(
        readFileSync(
          resolve(
            cacheDir,
            'v1',
            key.slice(0, 2),
            key.slice(2, 4),
            key,
            'index.json',
          ),
          'utf8',
        ),
      ) as { runtime: { requested: string; resolved: string } }
    })

    expect(keys).toHaveLength(2)
    expect(
      manifests.map(manifest => manifest.runtime.resolved).sort(),
    ).toStrictEqual(['native', 'wasm'])
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('uses legacy WOFF2 fallback as the pipeline runtime', async () => {
  const files = await optimize({
    input: [fixture],
    plugins: [ttf2woff2({ clone: false, fallback: 'wasm' })],
  })
  const woff2 = files.find(file => file.format === 'woff2')

  expect(
    Buffer.from(woff2?.contents ?? [])
      .subarray(0, 4)
      .toString('ascii'),
  ).toBe('wOF2')
  await expect(
    optimize({
      input: [fixture],
      runtime: 'native',
      plugins: [ttf2woff2({ fallback: 'wasm' })],
    }),
  ).rejects.toThrow('runtime `native` conflicts with WOFF2 fallback `wasm`')
})

it('reuses cached outputs for matching native optimize inputs', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-cache-'))
  const cacheDir = resolve(workDir, 'cache')
  const firstOutDir = resolve(workDir, 'first')
  const secondOutDir = resolve(workDir, 'second')
  const changedOutDir = resolve(workDir, 'changed')

  try {
    const firstFiles = await optimize({
      input: [fixture],
      outDir: firstOutDir,
      cache: { enabled: true, dir: cacheDir },
      plugins: [glyph({ text: 'Hello' }), ttf2woff({ clone: false })],
    })
    const secondFiles = await optimize({
      input: [fixture],
      outDir: secondOutDir,
      cache: { enabled: true, dir: cacheDir },
      plugins: [glyph({ text: 'Hello' }), ttf2woff({ clone: false })],
    })
    const changedFiles = await optimize({
      input: [fixture],
      outDir: changedOutDir,
      cache: { enabled: true, dir: cacheDir },
      plugins: [glyph({ text: 'World' }), ttf2woff({ clone: false })],
    })
    const firstWoff = firstFiles.find(
      file => file.path === 'roboto-regular.woff',
    )
    const secondWoff = secondFiles.find(
      file => file.path === 'roboto-regular.woff',
    )
    const changedWoff = changedFiles.find(
      file => file.path === 'roboto-regular.woff',
    )

    expect(firstWoff).toBeDefined()
    expect(secondWoff).toBeDefined()
    expect(changedWoff).toBeDefined()
    if (
      firstWoff === undefined ||
      secondWoff === undefined ||
      changedWoff === undefined
    ) {
      throw new Error('cache test did not emit expected WOFF assets')
    }

    expect(secondWoff.meta['cache']).toMatchObject({ hit: true })
    expect(changedWoff.meta['cache']).toBeUndefined()
    expect(Buffer.from(secondWoff.contents)).toStrictEqual(
      Buffer.from(firstWoff.contents),
    )
    expect(
      Buffer.from(
        readFileSync(resolve(secondOutDir, 'roboto-regular.woff')).subarray(
          0,
          4,
        ),
      ).toString('ascii'),
    ).toBe('wOFF')
    expect(
      readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'),
    ).toContain('roboto-regular.woff')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('loads a config file using the modern web preset', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-modern-config-'))
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.mjs')
  const packageEntry = resolve(currentDir, '../src/index.ts')

  writeFileSync(
    configPath,
    `import { modernWeb } from ${JSON.stringify(packageEntry)}

    export default {
      input: [${JSON.stringify(fixture)}],
      outDir: ${JSON.stringify(outputDir)},
      plugins: modernWeb({
        text: 'Hello',
        fontFamily: 'Roboto',
        fontPath: './',
      }),
    }`,
  )

  try {
    const config = await loadConfig(configPath)
    const files = await optimize(config)
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(cssAsset).toBeDefined()
    if (cssAsset === undefined) {
      throw new Error('modernWeb config did not emit CSS')
    }
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "url('./roboto-regular.woff') format('woff')",
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "url('./roboto-regular.woff2') format('woff2')",
    )
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})
