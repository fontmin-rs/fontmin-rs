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
import { expect, it } from 'vitest'
import { inspect, ttfToWoff } from '../src/index'
import {
  currentDir,
  fixture,
  bin,
  homeSvg,
  userSvg,
  flagsFromUsage,
} from './api-fixtures'

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

it('normalizes WOFF input through the package bin build pipeline', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-build-woff-'))
  const input = resolve(workDir, 'source.woff')
  const outputDir = resolve(workDir, 'dist')

  try {
    writeFileSync(input, ttfToWoff(readFileSync(fixture)))
    execFileSync(process.execPath, [
      bin,
      'build',
      input,
      '-o',
      outputDir,
      '--formats',
      'woff2',
    ])

    const output = readFileSync(resolve(outputDir, 'source.woff2'))

    expect(output.subarray(0, 4).toString('ascii')).toBe('wOF2')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
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

it('exposes the frozen public command surface through the package bin', () => {
  const contract = JSON.parse(
    readFileSync(
      resolve(currentDir, '../../../contracts/public-api.json'),
      'utf8',
    ),
  ) as {
    cli: {
      commands: Record<
        string,
        { flags: string[]; shortFlags: string[]; positionals: string[] }
      >
      globalFlags: string[]
    }
  }
  const help = execFileSync(process.execPath, [bin, '--help'], {
    encoding: 'utf8',
  })
  const commands = [
    ...help.matchAll(/^\s*fontmin-rs (?<command>[a-z]+)(?:\s|$)/gmu),
  ]
    .map(match => match.groups?.['command'])
    .filter(command => command !== undefined)

  expect([...new Set(commands)].toSorted()).toStrictEqual(
    Object.keys(contract.cli.commands).toSorted(),
  )

  for (const [command, surface] of Object.entries(contract.cli.commands)) {
    const commandUsage = help
      .split('\n')
      .filter(line => line.trimStart().startsWith(`fontmin-rs ${command}`))
      .join('\n')

    expect(flagsFromUsage(commandUsage)).toStrictEqual(
      [...surface.flags, ...surface.shortFlags].toSorted(),
    )
    for (const positional of surface.positionals) {
      expect(commandUsage.toUpperCase()).toContain(`<${positional}`)
    }
  }

  for (const flag of contract.cli.globalFlags) {
    expect(() => execFileSync(process.execPath, [bin, flag])).not.toThrow()
  }
})

it('applies CSS Unicode ranges and delivery slices through the package bin', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-delivery-'))
  const cssOutputDir = resolve(workDir, 'css')
  const deliveryOutputDir = resolve(workDir, 'delivery')

  try {
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      cssOutputDir,
      '--formats',
      'woff2,css',
      '--css-unicode-range',
      'U+0020-007E',
      '--css-unicode-range',
      'u+4e00-9fff',
    ])
    execFileSync(process.execPath, [
      bin,
      'build',
      fixture,
      '-o',
      deliveryOutputDir,
      '--formats',
      'woff2,css',
      '--delivery-slice',
      'latin:U+0041-004D',
      '--delivery-slice',
      'latin:U+004E-005A',
      '--delivery-slice',
      'digits:U+0030-0039',
    ])

    expect(
      readFileSync(resolve(cssOutputDir, 'roboto-regular.css'), 'utf8'),
    ).toContain('unicode-range: U+0020-007E, U+4E00-9FFF;')
    expect(
      existsSync(resolve(deliveryOutputDir, 'roboto-regular-latin.woff2')),
    ).toBe(true)
    expect(
      existsSync(resolve(deliveryOutputDir, 'roboto-regular-digits.woff2')),
    ).toBe(true)
    const deliveryCss = readFileSync(
      resolve(deliveryOutputDir, 'roboto-regular-latin.css'),
      'utf8',
    )

    expect(deliveryCss).toContain('unicode-range: U+0041-004D, U+004E-005A;')
    expect(deliveryCss).toContain('unicode-range: U+0030-0039;')
  } finally {
    rmSync(workDir, { recursive: true, force: true })
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

    writeFileSync(
      resolve(
        cacheDir,
        'v1',
        cacheKey.slice(0, 2),
        cacheKey.slice(2, 4),
        cacheKey,
        '000.ttf',
      ),
      sentinel,
    )
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

    writeFileSync(
      resolve(
        cacheDir,
        'v1',
        cacheKey.slice(0, 2),
        cacheKey.slice(2, 4),
        cacheKey,
        '000.ttf',
      ),
      sentinel,
    )
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

    writeFileSync(
      resolve(
        cacheDir,
        'v1',
        cacheKey.slice(0, 2),
        cacheKey.slice(2, 4),
        cacheKey,
        '000.woff',
      ),
      sentinel,
    )
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

    writeFileSync(
      resolve(
        cacheDir,
        'v1',
        cacheKey.slice(0, 2),
        cacheKey.slice(2, 4),
        cacheKey,
        '000.woff',
      ),
      sentinel,
    )
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

it('refuses to clean an output directory containing package bin inputs', () => {
  const workDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-bin-safe-clean-'))
  const inputDirectory = resolve(workDir, 'inputs')
  const inputPath = resolve(inputDirectory, 'font.ttf')
  const configPath = resolve(workDir, 'fontmin.config.json')

  mkdirSync(inputDirectory)
  writeFileSync(inputPath, readFileSync(fixture))
  writeFileSync(
    configPath,
    JSON.stringify({
      clean: true,
      input: ['inputs/font.ttf'],
      outDir: 'inputs',
      outputs: [{ format: 'ttf' }],
    }),
  )

  try {
    expect(() =>
      execFileSync(process.execPath, [bin, 'build', '--config', configPath], {
        stdio: 'pipe',
      }),
    ).toThrow('Command failed')
    expect(existsSync(inputPath)).toBe(true)
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
