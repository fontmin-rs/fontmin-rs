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
  defineConfig,
  definePlugin,
  glyph,
  loadConfig,
  optimize,
  otf2ttf,
  svg2ttf,
  svgs2ttf,
  ttf2woff2,
} from '../src/index'
import { fixture } from './api-fixtures'

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
