import { execFileSync } from 'node:child_process'
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import Fontmin, {
  css,
  defineConfig,
  definePlugin,
  eotToTtf,
  generateFontFaceCss,
  glyph,
  inspect,
  loadConfig,
  modernWeb,
  optimize,
  otf2ttf,
  otfToTtf,
  subsetTtf,
  svg2ttf,
  svgFontToTtf,
  svgs2ttf,
  svgsToTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
  woffToTtf,
} from '../src/index'

const currentDir = import.meta.dirname
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)
const bin = resolve(currentDir, '../bin/fontmin-rs.mjs')
const homeSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M100 500 L500 100 L900 500 L900 900 L100 900 Z"/></svg>'
const userSvg =
  '<svg viewBox="0 0 1000 1000"><path d="M500 100 C620 100 700 180 700 300 C700 420 620 500 500 500 C380 500 300 420 300 300 C300 180 380 100 500 100 Z M250 900 Q500 650 750 900 Z"/></svg>'
const svgFont =
  '<svg xmlns="http://www.w3.org/2000/svg"><defs><font id="icons" horiz-adv-x="1000"><font-face font-family="SVG Icons" units-per-em="1000" ascent="850" descent="-150" /><glyph glyph-name="home" unicode="&#xE101;" horiz-adv-x="1000" d="M100 100 L900 100 L900 900 L100 900 Z" /></font></defs></svg>'

function otfFromTtf(input: Buffer): Buffer {
  const otf = Buffer.from(input)

  otf.write('OTTO', 0, 'ascii')

  return otf
}

it('subsets through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello' })

  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('inspects TTF metadata through the public package api', () => {
  const input = readFileSync(fixture)
  const info = inspect(input)

  expect(info).toMatchObject({
    format: 'ttf',
    size: input.byteLength,
    metadata: {
      familyName: 'Roboto',
      subfamilyName: 'Regular',
      fullName: 'Roboto Regular',
      postScriptName: 'Roboto-Regular',
      glyphCount: 3387,
      unitsPerEm: 2048,
      ascender: 2146,
      descender: -555,
    },
  })
  expect(info.metadata.tables).toContain('name')
})

it('inspects OTF metadata through the public package api', () => {
  const input = otfFromTtf(readFileSync(fixture))
  const info = inspect(input)

  expect(info).toMatchObject({
    format: 'otf',
    size: input.byteLength,
    metadata: {
      familyName: 'Roboto',
      subfamilyName: 'Regular',
      fullName: 'Roboto Regular',
      postScriptName: 'Roboto-Regular',
      glyphCount: 3387,
      unitsPerEm: 2048,
      ascender: 2146,
      descender: -555,
    },
  })
  expect(info.metadata.tables).toContain('name')
})

it('reports unsupported OTF to TTF conversion through the public package api', () => {
  const input = otfFromTtf(readFileSync(fixture))

  expect(() => otfToTtf(input)).toThrow('unsupported font format: otf to ttf')
})

it('converts TTF to WOFF through the public package api', () => {
  const input = readFileSync(fixture)
  const output = ttfToWoff(input)

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOFF')
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('decodes WOFF to TTF through the public package api', () => {
  const input = readFileSync(fixture)
  const woff = ttfToWoff(input)
  const output = woffToTtf(woff)
  const info = inspect(output)

  expect(output.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Roboto')
})

it('inspects WOFF metadata through the public package api', () => {
  const input = readFileSync(fixture)
  const woff = ttfToWoff(input)
  const info = inspect(woff)

  expect(info.format).toBe('woff')
  expect(info.size).toBe(woff.byteLength)
  expect(info.metadata.fullName).toBe('Roboto Regular')
})

it('converts TTF to WOFF2 through the public package api', () => {
  const input = readFileSync(fixture)
  const output = ttfToWoff2(input)
  const declaredLength = output.readUInt32BE(8)

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
  expect(declaredLength).toBe(output.byteLength)
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('converts TTF to EOT through the public package api', () => {
  const input = readFileSync(fixture)
  const output = ttfToEot(input)

  expect(output.readUInt32LE(0)).toBe(output.byteLength)
  expect(output.readUInt32LE(4)).toBe(input.byteLength)
  expect(output.subarray(8, 12)).toEqual(Buffer.from([0x01, 0x00, 0x02, 0x00]))
  expect(output.subarray(34, 36)).toEqual(Buffer.from([0x4c, 0x50]))
  expect(output.subarray(output.byteLength - input.byteLength)).toEqual(input)
})

it('converts TTF to SVG through the public package api', () => {
  const input = readFileSync(fixture)
  const svg = ttfToSvg(input)

  expect(svg.startsWith('<svg')).toBe(true)
  expect(svg).toContain('<font ')
  expect(svg).toContain('font-family="Roboto"')
  expect(svg).toContain('unicode="A"')
  expect(svg).toContain('d="M')
})

it('combines SVG icons into a TTF through the public package api', () => {
  const output = svgsToTtf(
    [
      { name: 'home', contents: homeSvg, unicode: 0xe101 },
      { name: 'user', contents: userSvg },
    ],
    {
      fontName: 'Icon Set',
      startUnicode: 0xe200,
      ascent: 850,
      descent: -150,
      normalize: true,
    },
  )
  const info = inspect(output)

  expect(output.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Icon Set')
  expect(info.metadata.glyphCount).toBe(3)
  expect(info.metadata.unitsPerEm).toBe(1000)
})

it('converts an SVG font to a TTF through the public package api', () => {
  const output = svgFontToTtf(svgFont, { normalize: true, hinting: false })
  const info = inspect(output)

  expect(output.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('SVG Icons')
  expect(info.metadata.glyphCount).toBe(2)
  expect(info.metadata.unitsPerEm).toBe(1000)
})

it('decodes EOT to TTF through the public package api', () => {
  const input = readFileSync(fixture)
  const eot = ttfToEot(input)
  const output = eotToTtf(eot)
  const info = inspect(eot)

  expect(output.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
  expect(output.byteLength).toBe(input.byteLength)
  expect(info.format).toBe('eot')
  expect(info.metadata.fullName).toBe('Roboto Regular')
})

it('subsets a TTF through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-subset-'))
  const output = resolve(outputDir, 'roboto-subset.ttf')

  try {
    execFileSync(process.execPath, [
      bin,
      'subset',
      fixture,
      '-o',
      output,
      '--text',
      'Hello',
    ])

    expect(readFileSync(output).byteLength).toBeLessThan(
      readFileSync(fixture).byteLength,
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('converts a TTF through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-convert-'))
  const output = resolve(outputDir, 'roboto.woff2')

  try {
    execFileSync(process.execPath, [
      bin,
      'convert',
      fixture,
      '-f',
      'woff2',
      '-o',
      output,
    ])

    const contents = readFileSync(output)
    expect(contents.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(contents.byteLength).toBeLessThan(readFileSync(fixture).byteLength)
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('converts a TTF to EOT through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-eot-'))
  const output = resolve(outputDir, 'roboto.eot')

  try {
    execFileSync(process.execPath, [
      bin,
      'convert',
      fixture,
      '-f',
      'eot',
      '-o',
      output,
    ])

    const input = readFileSync(fixture)
    const contents = readFileSync(output)

    expect(contents.readUInt32LE(0)).toBe(contents.byteLength)
    expect(contents.readUInt32LE(4)).toBe(input.byteLength)
    expect(contents.subarray(8, 12)).toEqual(
      Buffer.from([0x01, 0x00, 0x02, 0x00]),
    )
    expect(contents.subarray(34, 36)).toEqual(Buffer.from([0x4c, 0x50]))
    expect(contents.subarray(contents.byteLength - input.byteLength)).toEqual(
      input,
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('converts a TTF to SVG through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-svg-'))
  const output = resolve(outputDir, 'roboto.svg')

  try {
    execFileSync(process.execPath, [
      bin,
      'convert',
      fixture,
      '-f',
      'svg',
      '-o',
      output,
    ])

    const svg = readFileSync(output, 'utf8')

    expect(svg.startsWith('<svg')).toBe(true)
    expect(svg).toContain('<font ')
    expect(svg).toContain('font-family="Roboto"')
    expect(svg).toContain('unicode="A"')
    expect(svg).toContain('d="M')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('decodes a WOFF through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-woff-'))
  const woff = resolve(outputDir, 'roboto.woff')
  const output = resolve(outputDir, 'roboto.ttf')

  try {
    execFileSync(process.execPath, [
      bin,
      'convert',
      fixture,
      '-f',
      'woff',
      '-o',
      woff,
    ])
    execFileSync(process.execPath, [
      bin,
      'convert',
      woff,
      '-f',
      'ttf',
      '-o',
      output,
    ])

    const contents = readFileSync(output)
    const info = inspect(contents)

    expect(contents.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
    expect(info.format).toBe('ttf')
    expect(info.metadata.fullName).toBe('Roboto Regular')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('decodes an EOT through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-eot-decode-'))
  const eot = resolve(outputDir, 'roboto.eot')
  const output = resolve(outputDir, 'roboto.ttf')

  try {
    execFileSync(process.execPath, [
      bin,
      'convert',
      fixture,
      '-f',
      'eot',
      '-o',
      eot,
    ])
    execFileSync(process.execPath, [
      bin,
      'convert',
      eot,
      '-f',
      'ttf',
      '-o',
      output,
    ])

    const contents = readFileSync(output)
    const info = inspect(readFileSync(eot))

    expect(contents.subarray(0, 4)).toEqual(Buffer.from([0, 1, 0, 0]))
    expect(info.format).toBe('eot')
    expect(info.metadata.fullName).toBe('Roboto Regular')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('rejects unsupported WOFF2 to TTF conversion through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-woff2-'))
  const woff2 = resolve(outputDir, 'roboto.woff2')
  const output = resolve(outputDir, 'roboto.ttf')
  let stderr = ''

  try {
    execFileSync(process.execPath, [
      bin,
      'convert',
      fixture,
      '-f',
      'woff2',
      '-o',
      woff2,
    ])

    try {
      execFileSync(process.execPath, [
        bin,
        'convert',
        woff2,
        '-f',
        'ttf',
        '-o',
        output,
      ])
    } catch (error) {
      stderr = String((error as { stderr?: Buffer }).stderr)
    }

    expect(stderr).toContain('unsupported input format for TTF conversion')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds EOT assets through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-build-eot-'))

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '--text',
      'Hello',
      '--formats',
      'eot,css',
      '--font-family',
      'Roboto',
    ])

    const eot = readFileSync(resolve(outputDir, 'roboto-regular.eot'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(eot.readUInt32LE(0)).toBe(eot.byteLength)
    expect(eot.subarray(8, 12)).toEqual(Buffer.from([0x01, 0x00, 0x02, 0x00]))
    expect(css).toContain("font-family: 'Roboto';")
    expect(css).toContain(
      "url('./roboto-regular.eot') format('embedded-opentype')",
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds SVG assets through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-build-svg-'))

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '--text',
      'Hello',
      '--formats',
      'svg,css',
      '--font-family',
      'Roboto',
    ])

    const svg = readFileSync(resolve(outputDir, 'roboto-regular.svg'), 'utf8')
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(svg).toContain('<font ')
    expect(svg).toContain('font-family="Roboto"')
    expect(svg).toContain('unicode="H"')
    expect(css).toContain("font-family: 'Roboto';")
    expect(css).toContain("url('./roboto-regular.svg') format('svg')")
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds modern web assets through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-build-'))

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '--text',
      'Hello',
      '--formats',
      'woff2,woff,css',
      '--font-family',
      'Roboto',
    ])

    const woff2 = readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(woff2.byteLength).toBeLessThan(readFileSync(fixture).byteLength)
    expect(woff.byteLength).toBeLessThan(readFileSync(fixture).byteLength)
    expect(css).toContain("font-family: 'Roboto';")
    expect(css).toContain("url('./roboto-regular.woff2') format('woff2')")
    expect(css).toContain("url('./roboto-regular.woff') format('woff')")
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds assets from a JSONC config through the package bin', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-config-'))
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      // JSONC keeps checked-in configs readable.
      "cwd": ${JSON.stringify(workDir)},
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff2" },
        { "format": "css" },
      ],
      "css": {
        "fontFamily": "Roboto Config",
        "fontPath": "/fonts",
        "local": false,
        "fontDisplay": "optional",
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    const woff2 = readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(woff2.byteLength).toBeLessThan(readFileSync(fixture).byteLength)
    expect(css).toContain("font-family: 'Roboto Config';")
    expect(css).toContain("url('/fonts/roboto-regular.woff2') format('woff2')")
    expect(css).toContain('font-display: optional;')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('discovers a JSONC config through the package bin build command', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-discover-'))
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff2" },
        { "format": "css" },
      ],
      "css": {
        "fontFamily": "Roboto Discovered",
        "fontPath": "./fonts",
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build'], { cwd: workDir })

    const woff2 = readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(css).toContain("font-family: 'Roboto Discovered';")
    expect(css).toContain("url('./fonts/roboto-regular.woff2') format('woff2')")
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('inspects a TTF as JSON through the package bin', () => {
  const output = execFileSync(process.execPath, [
    bin,
    'inspect',
    fixture,
    '--json',
  ])
  const info = JSON.parse(output.toString())

  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Roboto')
  expect(info.metadata.fullName).toBe('Roboto Regular')
})

it('inspects a WOFF as JSON through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-inspect-'))
  const woff = resolve(outputDir, 'roboto.woff')

  try {
    execFileSync(process.execPath, [
      bin,
      'convert',
      fixture,
      '-f',
      'woff',
      '-o',
      woff,
    ])

    const output = execFileSync(process.execPath, [
      bin,
      'inspect',
      woff,
      '--json',
    ])
    const info = JSON.parse(output.toString())

    expect(info.format).toBe('woff')
    expect(info.size).toBe(readFileSync(woff).byteLength)
    expect(info.metadata.familyName).toBe('Roboto')
    expect(info.metadata.fullName).toBe('Roboto Regular')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

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
          css({ fontFamily: 'Roboto', fontPath: './' }),
        ],
      }),
    )
    const paths = files.map(file => file.path).sort()
    const ttf = files.find(file => file.path === 'roboto-regular.ttf')
    const woff = files.find(file => file.path === 'roboto-regular.woff')
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(paths).toEqual([
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
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
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
      outputs: [{ format: 'woff2' }, { format: 'css' }],
      css: {
        fontDisplay: 'optional',
        fontFamily: 'Roboto Output',
        fontPath: '/fonts',
        local: false,
      },
    })
    const paths = files.map(file => file.path).sort()
    const woff2 = files.find(file => file.path === 'roboto-regular.woff2')
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(paths).toEqual(['roboto-regular.css', 'roboto-regular.woff2'])
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
      "url('/fonts/roboto-regular.woff2') format('woff2')",
    )
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff2')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOF2')
    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8'),
    ).toContain('font-display: optional;')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
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

    expect(paths).toEqual(['roboto-regular.ttf', 'roboto-regular.woff2'])
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

    expect(paths).toEqual(['roboto-regular.css', 'roboto-regular.eot'])
    expect(eot).toBeDefined()
    expect(cssAsset).toBeDefined()
    if (eot === undefined || cssAsset === undefined) {
      throw new Error('ttf2eot did not emit expected assets')
    }

    expect(Buffer.from(eot.contents).readUInt32LE(0)).toBe(
      eot.contents.byteLength,
    )
    expect(Buffer.from(eot.contents.subarray(8, 12))).toEqual(
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

    expect(paths).toEqual(['roboto-regular.css', 'roboto-regular.svg'])
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

it('reports unsupported OTF to TTF conversion through the builtin OTF plugin', async () => {
  const input = otfFromTtf(readFileSync(fixture))

  await expect(
    optimize({
      input: [input],
      plugins: [otf2ttf()],
    }),
  ).rejects.toThrow('unsupported font format: otf to ttf')
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

    expect(paths).toEqual(['icons.ttf'])
    expect(ttf).toBeDefined()
    if (ttf === undefined) {
      throw new Error('svg2ttf did not emit a TTF asset')
    }

    const info = inspect(ttf.contents)

    expect(Buffer.from(ttf.contents.subarray(0, 4))).toEqual(
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
          startUnicode: 0xe300,
          normalize: true,
        }),
        css({ fontFamily: 'pipe-icons', fontPath: './' }),
      ],
    })
    const paths = files.map(file => file.path).sort()
    const ttf = files.find(file => file.path === 'pipe-icons.ttf')
    const cssAsset = files.find(file => file.path === 'pipe-icons.css')

    expect(paths).toEqual(['pipe-icons.css', 'pipe-icons.ttf'])
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
    expect(readFileSync(resolve(outputDir, 'pipe-icons.ttf')).byteLength).toBe(
      ttf.contents.byteLength,
    )
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('returns typed config and plugin objects', () => {
  const config = defineConfig({
    input: ['fonts/*.ttf'],
    outDir: 'build',
    outputs: [{ format: 'woff2' }, 'css'],
    css: {
      fontDisplay: 'swap',
      fontFamily: 'Roboto',
      fontPath: './fonts',
    },
    plugins: [
      glyph({ text: 'Hello' }),
      otf2ttf(),
      ttf2woff2(),
      svg2ttf(),
      svgs2ttf(),
    ],
  })
  const plugin = definePlugin({ name: 'example' })

  expect(config.plugins).toHaveLength(5)
  expect(config.outputs).toHaveLength(2)
  expect(config.css?.fontDisplay).toBe('swap')
  expect(plugin.name).toBe('example')
})

it('provides filesystem and diagnostic helpers to custom plugins', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-plugin-context-'))
  const outputDir = resolve(workDir, 'dist')
  const seenNotes: string[] = []

  try {
    const files = await optimize({
      cwd: workDir,
      input: [fixture],
      outDir: outputDir,
      plugins: [
        definePlugin({
          name: 'context-probe',
          async buildStart(ctx) {
            expect(ctx.resolve('notes/plugin.txt')).toBe(
              resolve(workDir, 'notes/plugin.txt'),
            )
            await ctx.writeFile('notes/plugin.txt', 'plugin ready')
            ctx.warn('context warning')
          },
          async transform(asset, ctx) {
            const note = await ctx.readFile('notes/plugin.txt')

            seenNotes.push(note.toString('utf8'))
            ctx.emitFile({
              path: 'plugin-note.txt',
              contents: Buffer.from(
                ctx.diagnostics
                  .map(
                    diagnostic => `${diagnostic.level}:${diagnostic.message}`,
                  )
                  .join('\n'),
              ),
              format: 'unknown',
              sourceFormat: 'unknown',
              meta: { plugin: 'context-probe' },
            })

            return asset
          },
        }),
      ],
    })
    const emitted = files.find(file => file.path === 'plugin-note.txt')

    expect(seenNotes).toEqual(['plugin ready'])
    expect(emitted).toBeDefined()
    expect(readFileSync(resolve(workDir, 'notes/plugin.txt'), 'utf8')).toBe(
      'plugin ready',
    )
    expect(readFileSync(resolve(outputDir, 'plugin-note.txt'), 'utf8')).toBe(
      'warn:context warning',
    )
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('builds a fontmin-compatible chain', () => {
  const instance = new Fontmin()
    .src('fixtures/fonts/ttf/roboto-regular.ttf')
    .use(Fontmin.glyph({ text: 'Hello' }))
    .use(Fontmin.otf2ttf())
    .use(Fontmin.svg2ttf())
    .use(Fontmin.svgs2ttf({ fontName: 'icons' }))
    .dest('build')

  expect(instance.config()).toMatchObject({
    input: ['fixtures/fonts/ttf/roboto-regular.ttf'],
    outDir: 'build',
  })
})

it('runs a fontmin-compatible async chain', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-compat-'))

  try {
    const files = await new Fontmin()
      .src(fixture)
      .use(Fontmin.glyph({ text: 'Hello' }))
      .use(Fontmin.ttf2woff())
      .dest(outputDir)
      .runAsync()
    const woff = files.find(file => file.path === 'roboto-regular.woff')

    expect(woff).toBeDefined()
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('loads an ESM config file for optimize', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-config-'))
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.mjs')

  writeFileSync(
    configPath,
    `export default {
      input: [${JSON.stringify(fixture)}],
      outDir: ${JSON.stringify(outputDir)},
      plugins: [
        {
          name: 'fontmin:glyph',
          native: { kind: 'builtin', name: 'glyph', options: { text: 'Hello' } },
        },
        {
          name: 'fontmin:ttf2woff',
          native: { kind: 'builtin', name: 'ttf2woff', options: {} },
        },
      ],
    }`,
  )

  try {
    const config = await loadConfig(configPath)
    const files = await optimize(config)
    const woff = files.find(file => file.path === 'roboto-regular.woff')

    expect(woff).toBeDefined()
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('loads a JSONC config file for optimize', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-jsonc-config-'))
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      // comments and trailing commas should be accepted
      "input": [${JSON.stringify(fixture)}],
      "outDir": ${JSON.stringify(outputDir)},
      "plugins": [
        {
          "name": "fontmin:glyph",
          "native": { "kind": "builtin", "name": "glyph", "options": { "text": "Hello" } },
        },
        {
          "name": "fontmin:ttf2woff",
          "native": { "kind": "builtin", "name": "ttf2woff", "options": {} },
        },
      ],
    }`,
  )

  try {
    const config = await loadConfig(configPath)
    const files = await optimize(config)
    const woff = files.find(file => file.path === 'roboto-regular.woff')

    expect(woff).toBeDefined()
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('discovers a JSONC config file for optimize', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-jsonc-discover-'))
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')
  const originalCwd = process.cwd()

  writeFileSync(
    configPath,
    `{
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff" },
        { "format": "css" },
      ],
      "css": {
        "fontFamily": "Roboto Discovered API",
        "fontPath": "./assets",
      },
    }`,
  )

  try {
    process.chdir(workDir)
    const expectedCwd = process.cwd()
    const config = await loadConfig()
    process.chdir(originalCwd)

    const files = await optimize(config)
    const paths = files.map(file => file.path).sort()
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(config.cwd).toBe(expectedCwd)
    expect(paths).toEqual(['roboto-regular.css', 'roboto-regular.woff'])
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
    expect(css).toContain("font-family: 'Roboto Discovered API';")
    expect(css).toContain("url('./assets/roboto-regular.woff') format('woff')")
  } finally {
    process.chdir(originalCwd)
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('loads subset text from textFile in config for optimize', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-text-file-'))
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')
  const textPath = resolve(workDir, 'subset.txt')

  writeFileSync(textPath, 'Hello')
  writeFileSync(
    configPath,
    `{
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "textFile": "subset.txt" },
      "outputs": [
        { "format": "woff2" },
      ],
    }`,
  )

  try {
    const config = await loadConfig(configPath)
    const files = await optimize(config)
    const paths = files.map(file => file.path).sort()
    const woff2 = readFileSync(resolve(outputDir, 'roboto-regular.woff2'))

    expect(paths).toEqual(['roboto-regular.woff2'])
    expect(Buffer.from(woff2.subarray(0, 4)).toString('ascii')).toBe('wOF2')
    expect(woff2.byteLength).toBeLessThan(readFileSync(fixture).byteLength)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('expands glob input patterns in config for optimize', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-glob-input-'))
  const fontDir = resolve(workDir, 'fonts')
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  mkdirSync(fontDir, { recursive: true })
  writeFileSync(resolve(fontDir, 'roboto-a.ttf'), readFileSync(fixture))
  writeFileSync(resolve(fontDir, 'roboto-b.ttf'), readFileSync(fixture))
  writeFileSync(
    configPath,
    `{
      "input": ["fonts/*.ttf"],
      "outDir": "dist",
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff", "clone": false },
      ],
    }`,
  )

  try {
    const config = await loadConfig(configPath)
    const files = await optimize(config)
    const paths = files.map(file => file.path).sort()

    expect(paths).toEqual(['roboto-a.woff', 'roboto-b.woff'])
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-a.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-b.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('loads a JSONC output config file for optimize', async () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-jsonc-output-config-'),
  )
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      // Top-level outputs should configure the builtin output pipeline.
      "input": [${JSON.stringify(fixture)}],
      "outDir": ${JSON.stringify(outputDir)},
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff", "clone": false },
        { "format": "css" },
      ],
      "css": {
        "fontFamily": "Roboto JSONC Output",
        "fontPath": "./assets",
      },
    }`,
  )

  try {
    const config = await loadConfig(configPath)
    const files = await optimize(config)
    const paths = files.map(file => file.path).sort()
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(paths).toEqual(['roboto-regular.css', 'roboto-regular.woff'])
    expect(
      Buffer.from(
        readFileSync(resolve(outputDir, 'roboto-regular.woff')).subarray(0, 4),
      ).toString('ascii'),
    ).toBe('wOFF')
    expect(css).toContain("font-family: 'Roboto JSONC Output';")
    expect(css).toContain("url('./assets/roboto-regular.woff') format('woff')")
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

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

    expect(paths).toEqual([
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

it('inlines font assets when CSS base64 option is enabled', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-base64-css-'))

  try {
    const files = await optimize({
      input: [fixture],
      outDir: outputDir,
      plugins: [
        glyph({ text: 'Hello' }),
        ttf2woff({ clone: false }),
        css({ base64: true, fontFamily: 'Roboto' }),
      ],
    })
    const cssAsset = files.find(file => file.path === 'roboto-regular.css')

    expect(cssAsset).toBeDefined()
    if (cssAsset === undefined) {
      throw new Error('base64 CSS was not emitted')
    }

    const cssText = new TextDecoder().decode(cssAsset.contents)

    expect(cssText).toContain("url('data:font/woff;base64,")
    expect(cssText).not.toContain('roboto-regular.woff')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
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
    expect(Buffer.from(secondWoff.contents)).toEqual(
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
