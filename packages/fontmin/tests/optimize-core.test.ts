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
import { expect, it } from 'vitest'
import {
  css,
  deliverySlices,
  defineConfig,
  definePlugin,
  generateFontFaceCss,
  glyph,
  inspect,
  modernWeb,
  optimize,
  otf2ttf,
  svg2ttf,
  svgs2ttf,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
} from '../src/index'
import {
  fixture,
  cff2Fixture,
  homeSvg,
  userSvg,
  svgFont,
  otfFromTtf,
} from './api-fixtures'

it('generates @font-face CSS through the public package api', () => {
  const fontFaceCss = generateFontFaceCss(
    [
      { fileName: 'roboto.woff', format: 'woff' },
      { fileName: 'roboto.woff2', format: 'woff2' },
    ],
    {
      fontFamily: 'Roboto',
      fontPath: './fonts',
      local: true,
      fontDisplay: 'swap',
    },
  )

  expect(fontFaceCss).toContain('@font-face')
  expect(fontFaceCss).toContain("font-family: 'Roboto';")
  expect(fontFaceCss).toContain("url('./fonts/roboto.woff') format('woff')")
})

it('inlines @font-face CSS sources through the public package api', () => {
  const fontFaceCss = generateFontFaceCss(
    [
      {
        contents: new Uint8Array(Buffer.from('woff-bytes')),
        fileName: 'roboto.woff',
        format: 'woff',
      },
    ],
    {
      base64: true,
      fontFamily: 'Roboto',
      local: false,
    },
  )

  expect(fontFaceCss).toContain(
    "url('data:font/woff;base64,d29mZi1ieXRlcw==') format('woff')",
  )
  expect(fontFaceCss).not.toContain('roboto.woff')
})

it('emits unicode ranges through the public package api', () => {
  const css = generateFontFaceCss(
    [{ fileName: 'roboto.woff2', format: 'woff2' }],
    { unicodeRanges: ['U+0020-007E'] },
  )

  expect(css).toContain('unicode-range: U+0020-007E;')
})

it('serializes glyph unicode ranges in its built-in descriptor', () => {
  expect(
    JSON.parse(
      JSON.stringify(glyph({ unicodeRanges: ['U+0041-005A', 'U+0061-007A'] })),
    ),
  ).toStrictEqual({
    name: 'fontmin:glyph',
    native: {
      kind: 'builtin',
      name: 'glyph',
      options: {
        preserveHinting: false,
        unicodeRanges: ['U+0041-005A', 'U+0061-007A'],
      },
    },
  })
})

it('resolves @font-face CSS font family from source contents', () => {
  const fontFaceCss = generateFontFaceCss(
    [
      {
        contents: readFileSync(fixture),
        fileName: 'roboto.ttf',
        format: 'ttf',
      },
    ],
    {
      fontFamily: info => `${info.metadata.familyName} Source`,
      local: false,
    },
  )

  expect(fontFaceCss).toContain("font-family: 'Roboto Source';")
})

it('optimizes a TTF through builtin glyph, WOFF, and CSS plugins', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-'))

  try {
    const input = readFileSync(fixture)
    const files = await optimize(
      defineConfig({
        input: [fixture],
        outDir: outputDir,
        plugins: [
          glyph({ text: 'Hello' }),
          ttf2woff(),
          css({
            fontFamily: 'Roboto',
            fontPath: './',
            glyph: true,
            iconPrefix: 'icon',
          }),
        ],
      }),
    )
    const paths = files.map(file => file.path).sort()
    const ttf = files.find(file => file.path === 'roboto-regular.ttf')
    const woff = files.find(file => file.path === 'roboto-regular.woff')
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(paths).toStrictEqual([
      'roboto-regular.css',
      'roboto-regular.ttf',
      'roboto-regular.woff',
    ])
    expect(ttf).toBeDefined()
    expect(woff).toBeDefined()
    expect(cssAsset).toBeDefined()
    if (ttf === undefined || woff === undefined || cssAsset === undefined) {
      throw new Error('optimize did not emit expected assets')
    }
    expect(ttf.contents.byteLength).toBeLessThan(input.byteLength)
    expect(Buffer.from(woff.contents.subarray(0, 4)).toString('ascii')).toBe(
      'wOFF',
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "url('./roboto-regular.woff') format('woff')",
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      '.icon-u0048::before',
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      String.raw`content: '\0048';`,
    )
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('runs the complete file optimize pipeline through WASM', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-wasm-optimize-'))
  const transformedPaths: string[] = []

  try {
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      runtime: 'wasm',
      plugins: [
        definePlugin({
          name: 'wasm-context-probe',
          transform(asset, context) {
            transformedPaths.push(asset.path)
            context.emitFile({
              path: 'wasm-plugin.txt',
              contents: Buffer.from('custom plugin ran'),
              format: 'unknown',
              sourceFormat: asset.sourceFormat,
              meta: { plugin: 'wasm-context-probe' },
            })

            return asset
          },
        }),
        ...modernWeb({
          fallback: 'wasm',
          fontFamily: 'Roboto WASM',
          fontPath: './',
          text: 'Hello',
        }),
      ],
    })
    const woff = files.find(file => file.format === 'woff')
    const woff2 = files.find(file => file.format === 'woff2')
    const cssAsset = files.find(file => file.format === 'css')
    const emitted = files.find(file => file.path === 'wasm-plugin.txt')

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
    expect(transformedPaths).toStrictEqual(['roboto-regular.ttf'])
    expect(new TextDecoder().decode(emitted?.contents)).toBe(
      'custom plugin ran',
    )
    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
        .subarray(0, 4)
        .toString('ascii'),
    ).toBe('wOF2')
  } finally {
    rmSync(outputDir, { force: true, recursive: true })
  }
})

it('creates named Unicode delivery slices and CSS ranges', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-delivery-'))

  try {
    const files = await optimize(
      defineConfig({
        input: [fixture],
        outDir: outputDir,
        plugins: [
          deliverySlices([
            { name: 'latin-a-m', unicodeRanges: ['U+0041-004D'] },
            { name: 'latin-n-z', unicodeRanges: ['U+004E-005A'] },
          ]),
          ttf2woff2({ clone: false }),
          css({ fontFamily: 'Roboto', fontPath: './', local: false }),
        ],
      }),
    )
    const paths = files.map(file => file.path).sort()
    const cssAsset = files.find(
      file => file.path === 'roboto-regular-latin-a-m.css',
    )

    expect(paths).toStrictEqual([
      'roboto-regular-latin-a-m.css',
      'roboto-regular-latin-a-m.woff2',
      'roboto-regular-latin-n-z.woff2',
    ])
    expect(cssAsset).toBeDefined()
    if (cssAsset === undefined) {
      throw new Error('optimize did not emit delivery CSS')
    }

    const deliveryCss = new TextDecoder().decode(cssAsset.contents)

    expect(deliveryCss).toContain('unicode-range: U+0041-004D;')
    expect(deliveryCss).toContain('unicode-range: U+004E-005A;')
    expect(deliveryCss).toContain(
      "url('./roboto-regular-latin-a-m.woff2') format('woff2')",
    )
    expect(deliveryCss).toContain(
      "url('./roboto-regular-latin-n-z.woff2') format('woff2')",
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('rejects empty Unicode delivery slices', async () => {
  await expect(
    optimize({
      input: [fixture],
      plugins: [deliverySlices([])],
    }),
  ).rejects.toThrow('unicode delivery slices must not be empty')
})

it('clones TTF assets when the builtin glyph plugin clone option is enabled', async () => {
  const input = readFileSync(fixture)
  const files = await optimize({
    input: [fixture],
    plugins: [glyph({ text: 'Hello', clone: true })],
  })

  expect(files).toHaveLength(2)
  expect(files.map(file => file.path)).toStrictEqual([
    'roboto-regular.ttf',
    'roboto-regular.ttf',
  ])
  expect(files[0]?.contents.byteLength).toBe(input.byteLength)
  expect(files[1]?.contents.byteLength).toBeLessThan(input.byteLength)
})

it('uses CSS target as the optimized asset extension', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-css-target-'))

  try {
    const files = await optimize(
      defineConfig({
        input: [fixture],
        outDir: outputDir,
        plugins: [
          glyph({ text: 'Hello' }),
          ttf2woff2(),
          css({ target: 'less' }),
        ],
      }),
    )
    const paths = files.map(file => file.path).sort()
    const lessAsset = files.find(file => file.path === 'roboto-regular.less')

    expect(paths).toContain('roboto-regular.less')
    expect(paths).not.toContain('roboto-regular.css')
    expect(lessAsset).toBeDefined()
    if (lessAsset === undefined) {
      throw new Error('less CSS target was not emitted')
    }
    expect(new TextDecoder().decode(lessAsset.contents)).toContain('@font-face')
    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.less'), 'utf8'),
    ).toContain('@font-face')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('reports unavailable WOFF2 fallback modes through built-in plugins', async () => {
  await expect(
    optimize({
      input: [fixture],
      plugins: [ttf2woff2({ fallback: 'js' })],
    }),
  ).rejects.toThrow('WOFF2 fallback `js` is not available')
})

it('resolves CSS font family from font info in the JS pipeline', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-css-family-'))

  try {
    const files = await optimize(
      defineConfig({
        input: [fixture],
        outDir: outputDir,
        plugins: [
          glyph({ text: 'Hello' }),
          ttf2woff2(),
          css({
            fontFamily: info => `${info.metadata.familyName} Dynamic`,
            local: false,
          }),
        ],
      }),
    )
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(cssAsset).toBeDefined()
    if (cssAsset === undefined) {
      throw new Error('CSS asset was not emitted')
    }
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "font-family: 'Roboto Dynamic';",
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('optimizes top-level output and CSS config', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-output-config-'))

  try {
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      subset: { text: 'Hello' },
      outputs: [
        { fileName: 'webfont-modern.woff2', format: 'woff2' },
        { ext: 'module.css', format: 'css' },
      ],
      css: {
        fontDisplay: 'optional',
        fontFamily: 'Roboto Output',
        fontPath: '/fonts',
        local: false,
      },
    })
    const paths = files.map(file => file.path).sort()
    const woff2 = files.find(file => file.path === 'webfont-modern.woff2')
    const cssAsset = files.find(
      file => file.path === 'webfont-modern.module.css',
    )

    expect(paths).toStrictEqual([
      'webfont-modern.module.css',
      'webfont-modern.woff2',
    ])
    expect(woff2).toBeDefined()
    expect(cssAsset).toBeDefined()
    if (woff2 === undefined || cssAsset === undefined) {
      throw new Error('output config did not emit expected assets')
    }
    expect(Buffer.from(woff2.contents.subarray(0, 4)).toString('ascii')).toBe(
      'wOF2',
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "font-family: 'Roboto Output';",
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "url('/fonts/webfont-modern.woff2') format('woff2')",
    )
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'webfont-modern.woff2')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOF2')
    expect(
      readFileSync(resolve(outputDir, 'webfont-modern.module.css'), 'utf8'),
    ).toContain('font-display: optional;')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('cleans the configured output directory before writing optimized assets', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-clean-'))
  const outputDir = resolve(workDir, 'dist')

  mkdirSync(outputDir, { recursive: true })
  writeFileSync(resolve(outputDir, 'stale.woff'), 'stale')

  try {
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      clean: true,
      subset: { text: 'Hello' },
      outputs: [{ format: 'woff' }],
    })
    const paths = files.map(file => file.path)
    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))

    expect(paths).toStrictEqual(['roboto-regular.woff'])
    expect(existsSync(resolve(outputDir, 'stale.woff'))).toBe(false)
    expect(Buffer.from(woff.subarray(0, 4)).toString('ascii')).toBe('wOFF')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('optimizes a TTF through the builtin WOFF2 plugin', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-woff2-'))

  try {
    const input = readFileSync(fixture)
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      plugins: [glyph({ text: 'Hello' }), ttf2woff2()],
    })
    const paths = files.map(file => file.path).sort()
    const ttf = files.find(file => file.path === 'roboto-regular.ttf')
    const woff2 = files.find(file => file.path === 'roboto-regular.woff2')

    expect(paths).toStrictEqual(['roboto-regular.ttf', 'roboto-regular.woff2'])
    expect(ttf).toBeDefined()
    expect(woff2).toBeDefined()
    if (ttf === undefined || woff2 === undefined) {
      throw new Error('ttf2woff2 did not emit expected assets')
    }
    expect(ttf.contents.byteLength).toBeLessThan(input.byteLength)
    expect(Buffer.from(woff2.contents.subarray(0, 4)).toString('ascii')).toBe(
      'wOF2',
    )
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff2')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOF2')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('optimizes a TTF through the builtin EOT plugin', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-eot-'))

  try {
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      plugins: [
        glyph({ text: 'Hello' }),
        ttf2eot({ clone: false }),
        css({ fontFamily: 'Roboto', fontPath: './' }),
      ],
    })
    const paths = files.map(file => file.path).sort()
    const eot = files.find(file => file.path === 'roboto-regular.eot')
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(paths).toStrictEqual(['roboto-regular.css', 'roboto-regular.eot'])
    expect(eot).toBeDefined()
    expect(cssAsset).toBeDefined()
    if (eot === undefined || cssAsset === undefined) {
      throw new Error('ttf2eot did not emit expected assets')
    }

    expect(Buffer.from(eot.contents).readUInt32LE(0)).toBe(
      eot.contents.byteLength,
    )
    expect(Buffer.from(eot.contents.subarray(8, 12))).toStrictEqual(
      Buffer.from([0x01, 0x00, 0x02, 0x00]),
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "url('./roboto-regular.eot') format('embedded-opentype')",
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('optimizes a TTF through the builtin SVG plugin', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-svg-'))

  try {
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      plugins: [
        glyph({ text: 'Hello' }),
        ttf2svg({ clone: false }),
        css({ fontFamily: 'Roboto', fontPath: './' }),
      ],
    })
    const paths = files.map(file => file.path).sort()
    const svg = files.find(file => file.path === 'roboto-regular.svg')
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(paths).toStrictEqual(['roboto-regular.css', 'roboto-regular.svg'])
    expect(svg).toBeDefined()
    expect(cssAsset).toBeDefined()
    if (svg === undefined || cssAsset === undefined) {
      throw new Error('ttf2svg did not emit expected assets')
    }

    const svgText = new TextDecoder().decode(svg.contents)

    expect(svgText).toContain('<font ')
    expect(svgText).toContain('font-family="Roboto"')
    expect(svgText).toContain('unicode="H"')
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "url('./roboto-regular.svg') format('svg')",
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('converts glyf-backed OTF through the builtin OTF plugin', async () => {
  const input = otfFromTtf(readFileSync(fixture))

  const assets = await optimize({
    input: [input],
    plugins: [otf2ttf()],
  })

  expect(assets).toHaveLength(2)
  const source = assets[0]!
  const converted = assets[1]!

  expect(source.format).toBe('otf')
  expect(converted.format).toBe('ttf')
  expect(converted.path).toBe('fontmin.ttf')
  expect(converted.contents.subarray(0, 4)).toStrictEqual(
    Buffer.from([0, 1, 0, 0]),
  )
})

it('instantiates CFF2 coordinates through the builtin OTF plugin', async () => {
  const input = readFileSync(cff2Fixture)
  const assets = await optimize({
    input: [input],
    plugins: [
      otf2ttf({
        clone: false,
        variationCoordinates: { wght: 700, opsz: 14 },
      }),
    ],
  })

  expect(assets).toHaveLength(1)
  const converted = assets[0]!
  const info = inspect(converted.contents)

  expect(converted.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Source Serif 4 Variable')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).not.toContain('CFF2')
  expect(info.metadata.tables).not.toContain('fvar')
})

it('optimizes an SVG font through the builtin SVG to TTF plugin', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-svg2ttf-'))
  const outputDir = resolve(workDir, 'dist')
  const svgPath = resolve(workDir, 'icons.svg')

  writeFileSync(svgPath, svgFont)

  try {
    const files = await optimize({
      input: [svgPath],
      outDir: outputDir,
      plugins: [svg2ttf({ clone: false, normalize: true })],
    })
    const paths = files.map(file => file.path).sort()
    const ttf = files.find(file => file.path === 'icons.ttf')

    expect(paths).toStrictEqual(['icons.ttf'])
    expect(ttf).toBeDefined()
    if (ttf === undefined) {
      throw new Error('svg2ttf did not emit a TTF asset')
    }

    const info = inspect(ttf.contents)

    expect(Buffer.from(ttf.contents.subarray(0, 4))).toStrictEqual(
      Buffer.from([0, 1, 0, 0]),
    )
    expect(info.metadata.familyName).toBe('SVG Icons')
    expect(readFileSync(resolve(outputDir, 'icons.ttf')).byteLength).toBe(
      ttf.contents.byteLength,
    )
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('combines SVG icon inputs through the builtin SVGs to TTF plugin', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-svgs2ttf-'))
  const outputDir = resolve(workDir, 'dist')
  const homePath = resolve(workDir, 'home.svg')
  const userPath = resolve(workDir, 'user.svg')

  writeFileSync(homePath, homeSvg)
  writeFileSync(userPath, userSvg)

  try {
    const files = await optimize({
      input: [homePath, userPath],
      outDir: outputDir,
      plugins: [
        svgs2ttf({
          fontName: 'pipe-icons',
          startUnicode: 58_112,
          normalize: true,
        }),
        css({
          asFileName: true,
          fontFamily: 'pipe-icons',
          fontPath: './',
          glyph: true,
          iconPrefix: 'icon',
        }),
      ],
    })
    const paths = files.map(file => file.path).sort()
    const ttf = files.find(file => file.path === 'pipe-icons.ttf')
    const cssAsset = files.find(file => file.path === 'pipe-icons.css')

    expect(paths).toStrictEqual(['pipe-icons.css', 'pipe-icons.ttf'])
    expect(ttf).toBeDefined()
    expect(cssAsset).toBeDefined()
    if (ttf === undefined || cssAsset === undefined) {
      throw new Error('svgs2ttf did not emit expected assets')
    }

    const info = inspect(ttf.contents)

    expect(info.metadata.familyName).toBe('pipe-icons')
    expect(info.metadata.glyphCount).toBe(3)
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      "url('./pipe-icons.ttf') format('truetype')",
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      '.icon-home::before',
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      String.raw`content: '\E300';`,
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      '.icon-user::before',
    )
    expect(new TextDecoder().decode(cssAsset.contents)).toContain(
      String.raw`content: '\E301';`,
    )
    expect(readFileSync(resolve(outputDir, 'pipe-icons.ttf')).byteLength).toBe(
      ttf.contents.byteLength,
    )
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('uses the public default stem for SVG icon collections', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-svg-default-'))
  const outputDir = resolve(workDir, 'dist')
  const homePath = resolve(workDir, 'home.svg')

  writeFileSync(homePath, homeSvg)

  try {
    const files = await optimize({
      input: [homePath],
      outDir: outputDir,
      plugins: [svgs2ttf()],
    })

    expect(files.map(file => file.path)).toStrictEqual(['iconfont.ttf'])
    expect(existsSync(resolve(outputDir, 'iconfont.ttf'))).toBe(true)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})
