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
import { inflateSync } from 'node:zlib'
import { expect, it } from 'vitest'
import Fontmin, {
  css,
  deliverySlices,
  defineConfig,
  definePlugin,
  eotToTtf,
  generateFontFaceCss,
  glyph,
  fontminCompatPreset,
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
  ttfToWoff2Async,
  ttf2eot,
  ttf2svg,
  ttf2woff,
  ttf2woff2,
  validateWoff2,
  woff2ToTtf,
  woffToTtf,
} from '../src/index'
import { fontminCompatPreset as fontminCompatPresetFromSubpath } from '../src/presets'

const currentDir = import.meta.dirname
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)
const cffFixture = resolve(
  currentDir,
  '../../../fixtures/fonts/otf/source-sans-3-regular.otf',
)
const cff2Fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/otf/source-serif-4-variable-roman.otf',
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

it('subsets from Unicode ranges through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { unicodeRanges: ['U+0041-0042'] })

  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('subsets with text loaded from textFile through the public package api', () => {
  const dir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-'))
  const textFile = resolve(dir, 'glyphs.txt')
  const input = readFileSync(fixture)

  try {
    writeFileSync(textFile, 'Hello')

    const output = subsetTtf(input, { textFile })
    const expected = subsetTtf(input, { text: 'Hello' })

    expect(output).toStrictEqual(expected)
  } finally {
    rmSync(dir, { force: true, recursive: true })
  }
})

it('keeps original font data when trim is disabled through the public package api', () => {
  const input = readFileSync(fixture)
  const output = subsetTtf(input, { text: 'Hello', trim: false })

  expect(output.byteLength).toBe(input.byteLength)
  expect(Buffer.compare(output, input)).toBe(0)
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

it('converts glyf-backed OTF to TTF through the public package api', () => {
  const input = otfFromTtf(readFileSync(fixture))
  const output = otfToTtf(input)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(output.subarray(4)).toStrictEqual(input.subarray(4))
})

it('converts a real static CFF OTF to TTF through the public package api', () => {
  const output = otfToTtf(readFileSync(cffFixture))
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Source Sans 3')
  expect(info.metadata.tables).not.toContain('CFF ')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).toContain('GSUB')
  expect(info.metadata.tables).toContain('GPOS')
})

it('instantiates CFF2 coordinates through the public package api', () => {
  const output = otfToTtf(readFileSync(cff2Fixture), {
    variationCoordinates: { wght: 700, opsz: 14 },
  })
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Source Serif 4 Variable')
  expect(info.metadata.tables).toContain('glyf')
  expect(info.metadata.tables).not.toContain('CFF2')
  expect(info.metadata.tables).not.toContain('fvar')
  expect(info.metadata.tables).not.toContain('HVAR')
})

it('converts TTF to WOFF through the public package api', () => {
  const input = readFileSync(fixture)
  const output = ttfToWoff(input)

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOFF')
  expect(output.byteLength).toBeLessThan(input.byteLength)
})

it('converts TTF to WOFF with metadata and private data through the public package api', () => {
  const input = readFileSync(fixture)
  const metadata =
    '<?xml version="1.0" encoding="UTF-8"?><metadata version="1.0" />'
  const privateData = Buffer.from('fontmin-rs private data')
  const output = ttfToWoff(input, { metadata, privateData })
  const metaOffset = output.readUInt32BE(24)
  const metaLength = output.readUInt32BE(28)
  const metaOriginalLength = output.readUInt32BE(32)
  const privateOffset = output.readUInt32BE(36)
  const privateLength = output.readUInt32BE(40)
  const decoded = woffToTtf(output)

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOFF')
  expect(output.readUInt32BE(8)).toBe(output.byteLength)
  expect(metaOffset % 4).toBe(0)
  expect(privateOffset % 4).toBe(0)
  expect(metaOriginalLength).toBe(Buffer.byteLength(metadata))
  expect(
    inflateSync(output.subarray(metaOffset, metaOffset + metaLength)).toString(
      'utf8',
    ),
  ).toBe(metadata)
  expect(
    output.subarray(privateOffset, privateOffset + privateLength),
  ).toStrictEqual(privateData)
  expect(decoded.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
})

it('decodes WOFF to TTF through the public package api', () => {
  const input = readFileSync(fixture)
  const woff = ttfToWoff(input)
  const output = woffToTtf(woff)
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
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

it('inspects WOFF2 table metadata through the public package api', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)
  const info = inspect(woff2)

  expect(info.format).toBe('woff2')
  expect(info.size).toBe(woff2.byteLength)
  expect(info.metadata.familyName).toBe('Roboto')
  expect(info.metadata.fullName).toBe('Roboto Regular')
  expect(info.metadata.glyphCount).toBe(3387)
  expect(info.metadata.unitsPerEm).toBe(2048)
  expect(info.metadata.tables).toContain('cmap')
  expect(info.metadata.tables).toContain('name')
})

it('decodes WOFF2 to TTF through the public package api', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)
  const output = woff2ToTtf(woff2)
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Roboto')
  expect(info.metadata.glyphCount).toBe(3387)
})

it('validates WOFF2 data through the public package api', () => {
  const input = readFileSync(fixture)
  const woff2 = ttfToWoff2(input)

  expect(() => validateWoff2(woff2)).not.toThrow()
  expect(() => validateWoff2(Buffer.from('not woff2'))).toThrow(
    'expected WOFF2 data',
  )
})

it('uses native WOFF2 fallback modes through the public package api', () => {
  const input = readFileSync(fixture)

  for (const fallback of ['native', 'auto'] as const) {
    const output = ttfToWoff2(input, { fallback })

    expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
  }
})

it('reports unavailable non-native WOFF2 fallback modes through the public package api', () => {
  const input = readFileSync(fixture)

  expect(() => ttfToWoff2(input, { fallback: 'wasm' })).toThrow(
    'WOFF2 fallback `wasm` is asynchronous; use ttfToWoff2Async() instead.',
  )
  expect(() => ttfToWoff2(input, { fallback: 'js' })).toThrow(
    'WOFF2 fallback `js` is not available',
  )
})

it('encodes WOFF2 through the asynchronous WASM fallback without caller setup', async () => {
  const input = readFileSync(fixture)
  const output = await ttfToWoff2Async(input, { fallback: 'wasm' })

  expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
  expect(woff2ToTtf(output).subarray(0, 4)).toStrictEqual(
    Buffer.from([0, 1, 0, 0]),
  )
  await expect(ttfToWoff2Async(input, { fallback: 'js' })).rejects.toThrow(
    'WOFF2 fallback `js` is not available',
  )
})

it('labels WOFF2 WASM encoding failures', async () => {
  await expect(
    ttfToWoff2Async(Buffer.from('not a font'), { fallback: 'wasm' }),
  ).rejects.toThrow('WOFF2 WASM fallback failed')
})

it('converts TTF to EOT through the public package api', () => {
  const input = readFileSync(fixture)
  const output = ttfToEot(input)

  expect(output.readUInt32LE(0)).toBe(output.byteLength)
  expect(output.readUInt32LE(4)).toBe(input.byteLength)
  expect(output.subarray(8, 12)).toStrictEqual(
    Buffer.from([0x01, 0x00, 0x02, 0x00]),
  )
  expect(output.subarray(34, 36)).toStrictEqual(Buffer.from([0x4c, 0x50]))
  expect(output.subarray(output.byteLength - input.byteLength)).toStrictEqual(
    input,
  )
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
      { name: 'home', contents: homeSvg, unicode: 0xe1_01 },
      { name: 'user', contents: userSvg },
    ],
    {
      fontName: 'Icon Set',
      startUnicode: 0xe2_00,
      ascent: 850,
      descent: -150,
      normalize: true,
    },
  )
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(info.format).toBe('ttf')
  expect(info.metadata.familyName).toBe('Icon Set')
  expect(info.metadata.glyphCount).toBe(3)
  expect(info.metadata.unitsPerEm).toBe(1000)
})

it('converts an SVG font to a TTF through the public package api', () => {
  const output = svgFontToTtf(svgFont, { normalize: true, hinting: false })
  const info = inspect(output)

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
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

  expect(output.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
  expect(output.byteLength).toBe(input.byteLength)
  expect(info.format).toBe('eot')
  expect(info.metadata.fullName).toBe('Roboto Regular')
})

it('initializes a JSONC config through the package bin', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-init-'))
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  try {
    const stdout = execFileSync(process.execPath, [bin, 'init'], {
      cwd: workDir,
      encoding: 'utf8',
    })
    const config = readFileSync(configPath, 'utf8')

    expect(stdout).toContain('fontmin.config.jsonc')
    expect(config).toContain('Generated by fontmin-rs init')
    expect(config).toContain('"input"')
    expect(config).toContain('"outputs"')
    expect(config).toContain('"woff2"')
    expect(config).toContain('"fontDisplay": "swap"')
    expect(config).toContain('"cache"')

    mkdirSync(resolve(workDir, 'fonts'))
    writeFileSync(
      resolve(workDir, 'fonts/roboto-regular.ttf'),
      readFileSync(fixture),
    )
    execFileSync(process.execPath, [bin, 'build', '--config', configPath], {
      cwd: workDir,
    })

    const woff2 = readFileSync(resolve(workDir, 'build/roboto-regular.woff2'))
    const woff = readFileSync(resolve(workDir, 'build/roboto-regular.woff'))

    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(
      readFileSync(resolve(workDir, 'build/roboto-regular.css'), 'utf8'),
    ).toContain("font-family: 'MyFont';")
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('refuses to overwrite an existing JSONC config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-init-existing-'),
  )
  const configPath = resolve(workDir, 'fontmin.config.jsonc')
  let stderr = ''

  writeFileSync(configPath, 'keep me')

  try {
    try {
      execFileSync(process.execPath, [bin, 'init'], { cwd: workDir })
    } catch (error) {
      stderr = String((error as { stderr?: Buffer }).stderr)
    }

    expect(stderr).toContain('already exists')
    expect(readFileSync(configPath, 'utf8')).toBe('keep me')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
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

it('subsets a TTF from a text file through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-subset-file-'),
  )
  const textFile = resolve(outputDir, 'chars.txt')
  const output = resolve(outputDir, 'roboto-subset.ttf')

  writeFileSync(textFile, 'Hello')

  try {
    execFileSync(process.execPath, [
      bin,
      'subset',
      fixture,
      '-o',
      output,
      '--text-file',
      textFile,
    ])

    expect(readFileSync(output).byteLength).toBeLessThan(
      readFileSync(fixture).byteLength,
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('reports subset metrics from the package bin bench command', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-bench-'))
  const textFile = resolve(workDir, 'chars.txt')

  writeFileSync(textFile, 'Hello')

  try {
    const stdout = execFileSync(
      process.execPath,
      [bin, 'bench', fixture, '--text-file', textFile, '--json'],
      { encoding: 'utf8' },
    )
    const report = JSON.parse(stdout) as Record<string, unknown>

    expect(report['operation']).toBe('subset')
    expect(report['inputBytes']).toBe(readFileSync(fixture).byteLength)
    expect(Number(report['outputBytes'])).toBeLessThan(
      readFileSync(fixture).byteLength,
    )
    expect(Number.isInteger(report['elapsedMs'])).toBe(true)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('subsets a TTF from unicodes through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-subset-unicodes-'),
  )
  const output = resolve(outputDir, 'roboto-subset.ttf')

  try {
    execFileSync(process.execPath, [
      bin,
      'subset',
      fixture,
      '-o',
      output,
      '--unicodes',
      '0x48,0x65,0x6c,0x6f',
    ])

    expect(readFileSync(output).byteLength).toBeLessThan(
      readFileSync(fixture).byteLength,
    )
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('subsets a TTF with the basic text short flag through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-subset-basic-short-'),
  )
  const output = resolve(outputDir, 'roboto-subset.ttf')

  try {
    execFileSync(process.execPath, [bin, 'subset', fixture, '-o', output, '-b'])

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
    expect(contents.subarray(8, 12)).toStrictEqual(
      Buffer.from([0x01, 0x00, 0x02, 0x00]),
    )
    expect(contents.subarray(34, 36)).toStrictEqual(Buffer.from([0x4c, 0x50]))
    expect(
      contents.subarray(contents.byteLength - input.byteLength),
    ).toStrictEqual(input)
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

    expect(contents.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
    expect(info.format).toBe('ttf')
    expect(info.metadata.fullName).toBe('Roboto Regular')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('decodes a WOFF2 through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-woff2-'))
  const woff2 = resolve(outputDir, 'roboto.woff2')
  const output = resolve(outputDir, 'roboto.ttf')

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
    execFileSync(process.execPath, [
      bin,
      'convert',
      woff2,
      '-f',
      'ttf',
      '-o',
      output,
    ])

    const contents = readFileSync(output)
    const info = inspect(contents)

    expect(contents.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
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

    expect(contents.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
    expect(info.format).toBe('eot')
    expect(info.metadata.fullName).toBe('Roboto Regular')
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
    expect(eot.subarray(8, 12)).toStrictEqual(
      Buffer.from([0x01, 0x00, 0x02, 0x00]),
    )
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

it('reports elapsed time from the package bin build command', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-build-time-'))

  try {
    const stdout = execFileSync(
      process.execPath,
      [
        bin,
        'build',
        fixture,
        '-o',
        outputDir,
        '--text',
        'Hello',
        '--formats',
        'woff2,css',
        '-T',
      ],
      { encoding: 'utf8' },
    )

    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.woff2')).byteLength,
    ).toBeGreaterThan(0)
    expect(stdout).toContain('fontmin-rs build completed in ')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('suppresses elapsed time from the package bin build command', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-silent-'),
  )

  try {
    const stdout = execFileSync(
      process.execPath,
      [
        bin,
        'build',
        fixture,
        '-o',
        outputDir,
        '--text',
        'Hello',
        '--formats',
        'woff2,css',
        '-T',
        '--silent',
      ],
      { encoding: 'utf8' },
    )

    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.woff2')).byteLength,
    ).toBeGreaterThan(0)
    expect(stdout).toBe('')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('generates CSS glyph classes from a package bin flag', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-css-glyph-'),
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '--text',
      'Hi',
      '--formats',
      'woff,css',
      '--font-family',
      'Roboto',
      '--css-glyph',
    ])

    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(css).toContain('.icon-u0048::before')
    expect(css).toContain(String.raw`content: '\0048';`)
    expect(css).toContain('.icon-u0069::before')
    expect(css).toContain(String.raw`content: '\0069';`)
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds multiple font inputs through the package bin', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-build-many-'))
  const outputDir = resolve(workDir, 'dist')
  const firstInput = resolve(workDir, 'roboto-a.ttf')
  const secondInput = resolve(workDir, 'roboto-b.ttf')

  writeFileSync(firstInput, readFileSync(fixture))
  writeFileSync(secondInput, readFileSync(fixture))

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      firstInput,
      secondInput,
      '-o',
      outputDir,
      '--text',
      'Hello',
      '--formats',
      'woff',
    ])

    const firstWoff = readFileSync(resolve(outputDir, 'roboto-a.woff'))
    const secondWoff = readFileSync(resolve(outputDir, 'roboto-b.woff'))

    expect(firstWoff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(secondWoff.subarray(0, 4).toString('ascii')).toBe('wOFF')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('expands glob inputs through the package bin build command', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-build-glob-'))
  const fontDir = resolve(workDir, 'fonts')
  const outputDir = resolve(workDir, 'dist')

  mkdirSync(fontDir, { recursive: true })
  writeFileSync(resolve(fontDir, 'roboto-a.ttf'), readFileSync(fixture))
  writeFileSync(resolve(fontDir, 'roboto-b.ttf'), readFileSync(fixture))

  try {
    execFileSync(
      process.execPath,
      [
        bin,
        'build',
        'fonts/*.ttf',
        '-o',
        outputDir,
        '--text',
        'Hello',
        '--formats',
        'woff',
      ],
      { cwd: workDir },
    )

    const firstWoff = readFileSync(resolve(outputDir, 'roboto-a.woff'))
    const secondWoff = readFileSync(resolve(outputDir, 'roboto-b.woff'))

    expect(firstWoff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(secondWoff.subarray(0, 4).toString('ascii')).toBe('wOFF')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('builds WOFF assets with the deflate WOFF short flag through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-deflate-'),
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      '-d',
      fixture,
      '-o',
      outputDir,
      '--text',
      'Hello',
      '--formats',
      'woff',
    ])

    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))

    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(woff.byteLength).toBeLessThan(readFileSync(fixture).byteLength)
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('preserves requested TTF output through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-original-'))

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
      'ttf,woff,css',
      '--font-family',
      'Roboto',
    ])

    const ttf = readFileSync(resolve(outputDir, 'roboto-regular.ttf'))
    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(ttf.subarray(0, 4)).toStrictEqual(Buffer.from([0, 1, 0, 0]))
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(css).toContain("url('./roboto-regular.ttf') format('truetype')")
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds assets from a text file through the package bin', () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-build-file-'))
  const textFile = resolve(outputDir, 'chars.txt')

  writeFileSync(textFile, 'Hello')

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '--text-file',
      textFile,
      '--formats',
      'ttf',
    ])

    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.ttf')).byteLength,
    ).toBeLessThan(readFileSync(fixture).byteLength)
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds assets from unicodes through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-unicodes-'),
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '--unicodes',
      '0x48,0x65,0x6c,0x6f',
      '--formats',
      'ttf',
    ])

    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.ttf')).byteLength,
    ).toBeLessThan(readFileSync(fixture).byteLength)
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds assets with the basic text short flag through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-basic-short-'),
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '-b',
      '--formats',
      'ttf',
    ])

    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.ttf')).byteLength,
    ).toBeLessThan(readFileSync(fixture).byteLength)
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('drops requested TTF output with --no-original through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-no-original-'),
  )

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
      'ttf,woff,css',
      '--no-original',
      '--font-family',
      'Roboto',
    ])

    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(() =>
      readFileSync(resolve(outputDir, 'roboto-regular.ttf')),
    ).toThrow('ENOENT')
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(css).not.toContain('roboto-regular.ttf')
    expect(css).toContain("url('./roboto-regular.woff') format('woff')")
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds modern web assets from a preset through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-preset-modern-'),
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '--text',
      'Hello',
      '--preset',
      'modern-web',
      '--font-family',
      'Roboto',
    ])

    const woff2 = readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(css).toContain("font-family: 'Roboto';")
    expect(css).toContain("url('./roboto-regular.woff2') format('woff2')")
    expect(css).toContain("url('./roboto-regular.woff') format('woff')")
    expect(() =>
      readFileSync(resolve(outputDir, 'roboto-regular.eot')),
    ).toThrow('ENOENT')
    expect(() =>
      readFileSync(resolve(outputDir, 'roboto-regular.svg')),
    ).toThrow('ENOENT')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds compat assets from a preset through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-preset-compat-'),
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      outputDir,
      '--text',
      'Hello',
      '--preset',
      'compat',
      '--font-family',
      'Roboto Compat',
    ])

    const eot = readFileSync(resolve(outputDir, 'roboto-regular.eot'))
    const svg = readFileSync(resolve(outputDir, 'roboto-regular.svg'), 'utf8')
    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))
    const woff2 = readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(eot.subarray(8, 12)).toStrictEqual(
      Buffer.from([0x01, 0x00, 0x02, 0x00]),
    )
    expect(svg).toContain('<font ')
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(css).toContain("font-family: 'Roboto Compat';")
    expect(css).toContain('embedded-opentype')
    expect(css).toContain("format('svg')")
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('builds iconfont assets from a preset through the package bin', () => {
  const outputDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-preset-iconfont-'),
  )
  const home = resolve(outputDir, 'home.svg')
  const user = resolve(outputDir, 'user.svg')

  try {
    writeFileSync(home, homeSvg)
    writeFileSync(user, userSvg)

    execFileSync(process.execPath, [
      bin,
      'build',
      home,
      user,
      '-o',
      outputDir,
      '--preset',
      'iconfont',
      '--font-family',
      'Project Icons',
    ])

    const ttf = readFileSync(resolve(outputDir, 'iconfont.ttf'))
    const css = readFileSync(resolve(outputDir, 'iconfont.css'), 'utf8')
    const info = inspect(ttf)

    expect(ttf.subarray(0, 4)).toStrictEqual(
      Buffer.from([0x00, 0x01, 0x00, 0x00]),
    )
    expect(info.metadata.familyName).toBe('Project Icons')
    expect(info.metadata.glyphCount).toBe(3)
    expect(css).toContain("font-family: 'Project Icons';")
    expect(css).toContain("url('./iconfont.ttf') format('truetype')")
    expect(css).toContain('.icon-home::before')
    expect(css).toContain('.icon-user::before')
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('expands iconfont glob inputs through the package bin build command', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-build-preset-iconfont-glob-'),
  )
  const iconsDir = resolve(workDir, 'icons')
  const outputDir = resolve(workDir, 'dist')
  const home = resolve(iconsDir, 'home.svg')
  const user = resolve(iconsDir, 'user.svg')

  try {
    mkdirSync(iconsDir)
    writeFileSync(home, homeSvg)
    writeFileSync(user, userSvg)

    execFileSync(
      process.execPath,
      [
        bin,
        'build',
        'icons/*.svg',
        '-o',
        outputDir,
        '--preset',
        'iconfont',
        '--font-family',
        'Project Icons',
      ],
      { cwd: workDir },
    )

    const ttf = readFileSync(resolve(outputDir, 'iconfont.ttf'))
    const css = readFileSync(resolve(outputDir, 'iconfont.css'), 'utf8')
    const info = inspect(ttf)

    expect(ttf.subarray(0, 4)).toStrictEqual(
      Buffer.from([0x00, 0x01, 0x00, 0x00]),
    )
    expect(info.metadata.familyName).toBe('Project Icons')
    expect(info.metadata.glyphCount).toBe(3)
    expect(css).toContain('.icon-home::before')
    expect(css).toContain('.icon-user::before')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('builds iconfont assets from a config and preset through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-iconfont-'),
  )
  const home = resolve(workDir, 'home.svg')
  const user = resolve(workDir, 'user.svg')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')
  const outputDir = resolve(workDir, 'configured-icons')

  try {
    writeFileSync(home, homeSvg)
    writeFileSync(user, userSvg)
    writeFileSync(
      configPath,
      `{
        "cwd": ${JSON.stringify(workDir)},
        "input": ["home.svg", "user.svg"],
        "outDir": "configured-icons",
        "outputs": [
          { "format": "ttf", "fileName": "project-icons.ttf" },
          { "format": "css", "fileName": "project-icons.css" }
        ],
        "css": {
          "fontFamily": "Configured Icons",
          "fontPath": "/icons",
        },
      }`,
    )

    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '--preset',
      'iconfont',
    ])

    const ttf = readFileSync(resolve(outputDir, 'project-icons.ttf'))
    const css = readFileSync(resolve(outputDir, 'project-icons.css'), 'utf8')
    const info = inspect(ttf)

    expect(ttf.subarray(0, 4)).toStrictEqual(
      Buffer.from([0x00, 0x01, 0x00, 0x00]),
    )
    expect(info.metadata.familyName).toBe('Configured Icons')
    expect(info.metadata.glyphCount).toBe(3)
    expect(css).toContain("font-family: 'Configured Icons';")
    expect(css).toContain("url('/icons/project-icons.ttf') format('truetype')")
    expect(css).toContain('.icon-home::before')
    expect(css).toContain('.icon-user::before')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('reuses cached iconfont config outputs through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-iconfont-cache-'),
  )
  const home = resolve(workDir, 'home.svg')
  const user = resolve(workDir, 'user.svg')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')
  const outputDir = resolve(workDir, 'configured-icons')
  const cacheDir = resolve(workDir, 'cache')

  try {
    writeFileSync(home, homeSvg)
    writeFileSync(user, userSvg)
    writeFileSync(
      configPath,
      `{
        "cwd": ${JSON.stringify(workDir)},
        "input": ["home.svg", "user.svg"],
        "outDir": "configured-icons",
        "cache": { "enabled": true, "dir": "cache" },
        "outputs": [
          { "format": "ttf", "fileName": "project-icons.ttf" },
          { "format": "css", "fileName": "project-icons.css" }
        ],
        "css": {
          "fontFamily": "Configured Icons",
          "fontPath": "/icons",
        },
      }`,
    )

    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '--preset',
      'iconfont',
    ])

    const cacheIndex = JSON.parse(
      readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'),
    ) as { entries: Record<string, unknown> }
    const [cacheKey] = Object.keys(cacheIndex.entries)
    const sentinel = Buffer.from('cached-iconfont-output')

    if (cacheKey === undefined) {
      throw new Error('iconfont cache test did not write an index entry')
    }

    writeFileSync(resolve(cacheDir, 'v1', cacheKey, '000.ttf'), sentinel)
    rmSync(outputDir, { recursive: true, force: true })

    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '--preset',
      'iconfont',
    ])

    expect(readFileSync(resolve(outputDir, 'project-icons.ttf'))).toStrictEqual(
      sentinel,
    )
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('reuses cached direct iconfont outputs through the package bin --cache flag', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-iconfont-cache-'),
  )
  const iconsDir = resolve(workDir, 'icons')
  const outputDir = resolve(workDir, 'dist')
  const cacheDir = resolve(workDir, 'node_modules/.cache/fontmin-rs')

  try {
    mkdirSync(iconsDir)
    writeFileSync(resolve(iconsDir, 'home.svg'), homeSvg)
    writeFileSync(resolve(iconsDir, 'user.svg'), userSvg)

    execFileSync(
      process.execPath,
      [
        bin,
        'build',
        'icons/*.svg',
        '-o',
        outputDir,
        '--preset',
        'iconfont',
        '--cache',
      ],
      { cwd: workDir },
    )

    const cacheIndex = JSON.parse(
      readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'),
    ) as { entries: Record<string, unknown> }
    const [cacheKey] = Object.keys(cacheIndex.entries)
    const sentinel = Buffer.from('cached-direct-iconfont-output')

    if (cacheKey === undefined) {
      throw new Error('direct iconfont cache test did not write an index entry')
    }

    writeFileSync(resolve(cacheDir, 'v1', cacheKey, '000.ttf'), sentinel)
    rmSync(outputDir, { recursive: true, force: true })

    execFileSync(
      process.execPath,
      [
        bin,
        'build',
        'icons/*.svg',
        '-o',
        outputDir,
        '--preset',
        'iconfont',
        '--cache',
      ],
      { cwd: workDir },
    )

    expect(readFileSync(resolve(outputDir, 'iconfont.ttf'))).toStrictEqual(
      sentinel,
    )
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('uses iconfont CSS target with configured TTF name through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-iconfont-target-'),
  )
  const home = resolve(workDir, 'home.svg')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')
  const outputDir = resolve(workDir, 'configured-icons')

  try {
    writeFileSync(home, homeSvg)
    writeFileSync(
      configPath,
      `{
        "cwd": ${JSON.stringify(workDir)},
        "input": ["home.svg"],
        "outDir": "configured-icons",
        "outputs": [
          { "format": "ttf", "fileName": "project-icons.ttf" },
          { "format": "css" }
        ],
        "css": {
          "fontFamily": "Configured Icons",
          "fontPath": "/icons",
          "fontDisplay": "optional",
          "iconPrefix": "glyph",
          "local": false,
          "target": "less",
        },
      }`,
    )

    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '--preset',
      'iconfont',
    ])

    const less = readFileSync(resolve(outputDir, 'project-icons.less'), 'utf8')

    expect(existsSync(resolve(outputDir, 'iconfont.css'))).toBe(false)
    expect(less).toContain("font-family: 'Configured Icons';")
    expect(less).not.toContain('local(')
    expect(less).toContain('font-display: optional;')
    expect(less).toContain("url('/icons/project-icons.ttf') format('truetype')")
    expect(less).toContain('.glyph-home::before')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('inlines iconfont CSS sources from a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-iconfont-base64-'),
  )
  const home = resolve(workDir, 'home.svg')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')
  const outputDir = resolve(workDir, 'configured-icons')

  try {
    writeFileSync(home, homeSvg)
    writeFileSync(
      configPath,
      `{
        "cwd": ${JSON.stringify(workDir)},
        "input": ["home.svg"],
        "outDir": "configured-icons",
        "outputs": [
          { "format": "ttf", "fileName": "project-icons.ttf" },
          { "format": "css", "fileName": "project-icons.css" }
        ],
        "css": {
          "base64": true,
          "fontFamily": "Configured Icons",
          "fontPath": "/icons",
        },
      }`,
    )

    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '--preset',
      'iconfont',
    ])

    const ttf = readFileSync(resolve(outputDir, 'project-icons.ttf'))
    const css = readFileSync(resolve(outputDir, 'project-icons.css'), 'utf8')

    expect(ttf.subarray(0, 4)).toStrictEqual(
      Buffer.from([0x00, 0x01, 0x00, 0x00]),
    )
    expect(css).toContain("url('data:font/ttf;base64,")
    expect(css).not.toContain('/icons/project-icons.ttf')
    expect(css).toContain('.icon-home::before')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('honors iconfont CSS class naming from a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-iconfont-class-'),
  )
  const home = resolve(workDir, 'home.svg')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')
  const outputDir = resolve(workDir, 'configured-icons')

  try {
    writeFileSync(home, homeSvg)
    writeFileSync(
      configPath,
      `{
        "cwd": ${JSON.stringify(workDir)},
        "input": ["home.svg"],
        "outDir": "configured-icons",
        "outputs": [
          { "format": "ttf" },
          { "format": "css" }
        ],
        "css": {
          "asFileName": false,
          "fontFamily": "Configured Icons",
          "iconPrefix": "glyph",
        },
      }`,
    )

    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '--preset',
      'iconfont',
    ])

    const css = readFileSync(resolve(outputDir, 'iconfont.css'), 'utf8')

    expect(css).toContain('.glyph-uE001::before')
    expect(css).not.toContain('.glyph-home::before')
    expect(css).toContain(String.raw`content: '\E001';`)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
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

it('reuses cached config outputs through the package bin', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-config-cache-'))
  const outputDir = resolve(workDir, 'dist')
  const cacheDir = resolve(workDir, 'cache')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      "cwd": ${JSON.stringify(workDir)},
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "cache": { "enabled": true, "dir": "cache" },
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff" },
      ],
    }`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    const cacheIndex = JSON.parse(
      readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'),
    ) as { entries: Record<string, unknown> }
    const [cacheKey] = Object.keys(cacheIndex.entries)
    const sentinel = Buffer.from('cached-bin-output')

    if (cacheKey === undefined) {
      throw new Error('cache test did not write an index entry')
    }

    writeFileSync(resolve(cacheDir, 'v1', cacheKey, '000.woff'), sentinel)
    rmSync(outputDir, { recursive: true, force: true })

    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    expect(
      readFileSync(resolve(outputDir, 'roboto-regular.woff')),
    ).toStrictEqual(sentinel)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('reuses cached direct outputs through the package bin --cache flag', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-cache-'))
  const input = resolve(workDir, 'roboto.ttf')
  const outputDir = resolve(workDir, 'dist')
  const cacheDir = resolve(workDir, 'node_modules/.cache/fontmin-rs')

  try {
    writeFileSync(input, readFileSync(fixture))

    execFileSync(
      process.execPath,
      [
        bin,
        'build',
        'roboto.ttf',
        '-o',
        outputDir,
        '--formats',
        'woff',
        '--cache',
      ],
      { cwd: workDir },
    )

    const cacheIndex = JSON.parse(
      readFileSync(resolve(cacheDir, 'v1', 'index.json'), 'utf8'),
    ) as { entries: Record<string, unknown> }
    const [cacheKey] = Object.keys(cacheIndex.entries)
    const sentinel = Buffer.from('cached-direct-output')

    if (cacheKey === undefined) {
      throw new Error('direct cache test did not write an index entry')
    }

    writeFileSync(resolve(cacheDir, 'v1', cacheKey, '000.woff'), sentinel)
    rmSync(outputDir, { recursive: true, force: true })

    execFileSync(
      process.execPath,
      [
        bin,
        'build',
        'roboto.ttf',
        '-o',
        outputDir,
        '--formats',
        'woff',
        '--cache',
      ],
      { cwd: workDir },
    )

    expect(readFileSync(resolve(outputDir, 'roboto.woff'))).toStrictEqual(
      sentinel,
    )
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('lets --no-cache disable config cache through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-no-cache-'),
  )
  const cacheDir = resolve(workDir, 'cache')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      "cwd": ${JSON.stringify(workDir)},
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "cache": { "enabled": true, "dir": "cache" },
      "outputs": [
        { "format": "woff" },
      ],
    }`,
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '--no-cache',
    ])

    expect(existsSync(resolve(cacheDir, 'v1', 'index.json'))).toBe(false)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('inlines CSS font sources from a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-base64-'),
  )
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      "cwd": ${JSON.stringify(workDir)},
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff" },
        { "format": "css" },
      ],
      "css": {
        "base64": true,
        "fontFamily": "Roboto Inline",
        "fontPath": "/fonts",
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(css).toContain("font-family: 'Roboto Inline';")
    expect(css).toContain("url('data:font/woff;base64,")
    expect(css).not.toContain('/fonts/roboto-regular.woff')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('emits CSS glyph classes from a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-glyph-css-'),
  )
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      "cwd": ${JSON.stringify(workDir)},
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "text": "Hi" },
      "outputs": [
        { "format": "woff" },
        { "format": "css" },
      ],
      "css": {
        "fontFamily": "Roboto Glyph",
        "fontPath": "/fonts",
        "glyph": true,
        "iconPrefix": "glyph",
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(css).toContain('.glyph-u0048::before')
    expect(css).toContain(String.raw`content: '\0048';`)
    expect(css).toContain('.glyph-u0069::before')
    expect(css).toContain(String.raw`content: '\0069';`)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('honors output file names from a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-output-names-'),
  )
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      "cwd": ${JSON.stringify(workDir)},
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff2", "fileName": "webfont-modern.woff2" },
        { "format": "css", "ext": "module.css" },
      ],
      "css": {
        "fontFamily": "Roboto Output Bin",
        "fontPath": "/fonts",
        "local": false,
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    const woff2 = readFileSync(resolve(outputDir, 'webfont-modern.woff2'))
    const css = readFileSync(
      resolve(outputDir, 'webfont-modern.module.css'),
      'utf8',
    )

    expect(existsSync(resolve(outputDir, 'roboto-regular.woff2'))).toBe(false)
    expect(existsSync(resolve(outputDir, 'roboto-regular.css'))).toBe(false)
    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(css).toContain("font-family: 'Roboto Output Bin';")
    expect(css).toContain("url('/fonts/webfont-modern.woff2') format('woff2')")
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('uses CSS target as the config output extension through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-css-target-'),
  )
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      "cwd": ${JSON.stringify(workDir)},
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff2" },
        { "format": "css" },
      ],
      "css": {
        "fontFamily": "Roboto Less Bin",
        "target": "less",
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    const less = readFileSync(resolve(outputDir, 'roboto-regular.less'), 'utf8')

    expect(existsSync(resolve(outputDir, 'roboto-regular.css'))).toBe(false)
    expect(less).toContain("font-family: 'Roboto Less Bin';")
    expect(less).toContain('@font-face')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('applies CLI overrides when building from a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-overrides-'),
  )
  const outputDir = resolve(workDir, 'cli-dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
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
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '-o',
      'cli-dist',
      '--formats',
      'woff,css',
      '--text',
      'A',
      '--font-family',
      'Roboto CLI',
      '--font-path',
      '/cli',
    ])

    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')
    const info = inspect(woff)

    expect(existsSync(resolve(outputDir, 'roboto-regular.woff2'))).toBe(false)
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(info.metadata.glyphCount).toBe(2)
    expect(css).toContain("font-family: 'Roboto CLI';")
    expect(css).toContain("url('/cli/roboto-regular.woff') format('woff')")
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('applies a CSS glyph CLI override when building from a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-css-glyph-'),
  )
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(
    configPath,
    `{
      "cwd": ${JSON.stringify(workDir)},
      "input": [${JSON.stringify(fixture)}],
      "outDir": "dist",
      "subset": { "text": "Hi" },
      "outputs": [
        { "format": "woff" },
        { "format": "css" },
      ],
      "css": {
        "fontFamily": "Roboto Config",
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      '--css-glyph',
    ])

    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(css).toContain('.icon-u0048::before')
    expect(css).toContain(String.raw`content: '\0048';`)
    expect(css).toContain('.icon-u0069::before')
    expect(css).toContain(String.raw`content: '\0069';`)
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('uses CLI inputs when building from a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-config-inputs-'),
  )
  const configInput = resolve(workDir, 'config-font.ttf')
  const cliInput = resolve(workDir, 'cli-font.ttf')
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.jsonc')

  writeFileSync(configInput, readFileSync(fixture))
  writeFileSync(cliInput, readFileSync(fixture))
  writeFileSync(
    configPath,
    `{
      "cwd": ${JSON.stringify(workDir)},
      "input": ["config-font.ttf"],
      "outDir": "dist",
      "subset": { "text": "Hello" },
      "outputs": [
        { "format": "woff" },
        { "format": "css" },
      ],
      "css": {
        "fontFamily": "Roboto CLI Input",
        "fontPath": "/fonts",
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      '--config',
      configPath,
      'cli-font.ttf',
    ])

    const woff = readFileSync(resolve(outputDir, 'cli-font.woff'))
    const css = readFileSync(resolve(outputDir, 'cli-font.css'), 'utf8')

    expect(existsSync(resolve(outputDir, 'config-font.woff'))).toBe(false)
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(css).toContain("url('/fonts/cli-font.woff') format('woff')")
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('builds assets from an MJS config through the package bin', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-config-mjs-'))
  const inputPath = resolve(workDir, 'roboto-regular.ttf')
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.mjs')

  writeFileSync(inputPath, readFileSync(fixture))
  writeFileSync(
    configPath,
    `export default {
      input: ['roboto-regular.ttf'],
      outDir: 'dist',
      subset: { text: 'Hello' },
      outputs: [
        { format: 'woff2' },
        { format: 'css' },
      ],
      css: {
        fontFamily: 'Roboto MJS',
        fontPath: '/assets',
      },
    }`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    const woff2 = readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(css).toContain("font-family: 'Roboto MJS';")
    expect(css).toContain("url('/assets/roboto-regular.woff2') format('woff2')")
  } finally {
    rmSync(workDir, { recursive: true, force: true })
  }
})

it('builds assets from a TypeScript config through the package bin', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-config-ts-'))
  const inputPath = resolve(workDir, 'roboto-regular.ttf')
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.ts')

  writeFileSync(inputPath, readFileSync(fixture))
  writeFileSync(
    configPath,
    `const config: {
      input: string[]
      outDir: string
      subset: { text: string }
      outputs: Array<{ format: 'woff2' | 'css' }>
      css: { fontFamily: string; fontPath: string }
    } = {
      input: ['roboto-regular.ttf'],
      outDir: 'dist',
      subset: { text: 'Hello' },
      outputs: [
        { format: 'woff2' },
        { format: 'css' },
      ],
      css: {
        fontFamily: 'Roboto TS',
        fontPath: '/assets',
      },
    }

    export default config`,
  )

  try {
    execFileSync(process.execPath, [bin, 'build', '--config', configPath])

    const woff2 = readFileSync(resolve(outputDir, 'roboto-regular.woff2'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')

    expect(woff2.subarray(0, 4).toString('ascii')).toBe('wOF2')
    expect(css).toContain("font-family: 'Roboto TS';")
    expect(css).toContain("url('/assets/roboto-regular.woff2') format('woff2')")
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

it('applies CLI overrides when discovering a config through the package bin', () => {
  const workDir = mkdtempSync(
    resolve(tmpdir(), 'fontmin-rs-bin-discover-overrides-'),
  )
  const outputDir = resolve(workDir, 'cli-dist')
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
    execFileSync(
      process.execPath,
      [
        bin,
        'build',
        '-o',
        'cli-dist',
        '--formats',
        'woff,css',
        '--text',
        'A',
        '--font-family',
        'Roboto CLI',
        '--font-path',
        '/cli',
      ],
      { cwd: workDir },
    )

    const woff = readFileSync(resolve(outputDir, 'roboto-regular.woff'))
    const css = readFileSync(resolve(outputDir, 'roboto-regular.css'), 'utf8')
    const info = inspect(woff)

    expect(existsSync(resolve(outputDir, 'roboto-regular.woff2'))).toBe(false)
    expect(woff.subarray(0, 4).toString('ascii')).toBe('wOFF')
    expect(info.metadata.glyphCount).toBe(2)
    expect(css).toContain("font-family: 'Roboto CLI';")
    expect(css).toContain("url('/cli/roboto-regular.woff') format('woff')")
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

    expect(seenNotes).toStrictEqual(['plugin ready'])
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

it('loads a TypeScript config file for optimize', async () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-ts-config-'))
  const outputDir = resolve(workDir, 'dist')
  const configPath = resolve(workDir, 'fontmin.config.ts')

  writeFileSync(
    configPath,
    `const config: {
      input: string[]
      outDir: string
      plugins: Array<{
        name: string
        native: {
          kind: 'builtin'
          name: 'glyph' | 'ttf2woff'
          options: Record<string, unknown>
        }
      }>
    } = {
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
    }

    export default config`,
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
    expect(paths).toStrictEqual(['roboto-regular.css', 'roboto-regular.woff'])
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

    expect(paths).toStrictEqual(['roboto-regular.woff2'])
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

    expect(paths).toStrictEqual(['roboto-a.woff', 'roboto-b.woff'])
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

    expect(paths).toStrictEqual(['roboto-regular.css', 'roboto-regular.woff'])
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

it('runs the complete file optimize pipeline through WASM', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-wasm-optimize-'))
  const transforms: string[] = []

  try {
    const files = await optimize({
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
