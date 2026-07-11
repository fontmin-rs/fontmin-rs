#!/usr/bin/env node

import { mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises'
import { basename, dirname, extname, join, resolve } from 'node:path'
import {
  eotToTtf,
  generateFontFaceCss,
  inspectFont,
  subsetTtf,
  ttfToEot,
  ttfToSvg,
  ttfToWoff,
  ttfToWoff2,
  woffToTtf,
} from '@fontmin-rs/binding'
import { parse as parseJsonc } from 'jsonc-parser'

const DEFAULT_CONFIG_FILES = [
  'fontmin.config.ts',
  'fontmin.config.mts',
  'fontmin.config.mjs',
  'fontmin.config.cjs',
  'fontmin.config.json',
  'fontmin.config.jsonc',
]

const argv = process.argv.slice(2)
const command = argv[0]
const commandArgs = argv.slice(1)

try {
  if (command === 'subset') {
    await subsetCommand(commandArgs)
  } else if (command === 'convert') {
    await convertCommand(commandArgs)
  } else if (command === 'build') {
    await buildCommand(commandArgs)
  } else if (command === 'inspect') {
    await inspectCommand(commandArgs)
  } else {
    usage()
    process.exitCode = 1
  }
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
}

async function subsetCommand(args) {
  const output = readOption(args, ['-o', '--output'])
  const text = readOption(args, ['-t', '--text'])
  const basicText = readFlag(args, ['--basic-text'])
  const [input] = args

  requireValue(input, 'subset requires an input font')
  requireValue(output, 'subset requires -o, --output')
  if (text === undefined && !basicText) {
    throw new Error('subset requires --text or --basic-text')
  }

  const contents = await readFile(input)
  const subset = subsetTtf(contents, {
    basicText,
    text,
  })

  await writeOutput(output, subset)
}

async function convertCommand(args) {
  const output = readOption(args, ['-o', '--output'])
  const format = readOption(args, ['-f', '--format'])
  const [input] = args

  requireValue(input, 'convert requires an input font')
  requireValue(output, 'convert requires -o, --output')
  requireValue(format, 'convert requires -f, --format')

  const contents = await readFile(input)
  const converted = convertFont(contents, format)

  await writeOutput(output, converted)
}

async function buildCommand(args) {
  const configPath = readOption(args, ['-c', '--config'])
  const [input] = args

  if (configPath !== undefined) {
    await buildConfigCommand(configPath)
    return
  }

  if (input === undefined) {
    await buildConfigCommand(await findConfig())
    return
  }

  const outDir = readOption(args, ['-o', '--out-dir'])
  const text = readOption(args, ['-t', '--text'])
  const formats = readOption(args, ['--formats'])
  const fontFamily = readOption(args, ['--font-family'])
  const fontPath = readOption(args, ['--font-path']) ?? './'

  requireValue(input, 'build requires an input font')
  requireValue(outDir, 'build requires -o, --out-dir')
  requireValue(formats, 'build requires --formats')

  const contents = await readFile(input)
  const source =
    text === undefined
      ? contents
      : subsetTtf(contents, {
          text,
        })
  const baseName = basename(input, extname(input))
  const outputFormats = parseFormats(formats)
  const cssSources = []

  await mkdir(outDir, { recursive: true })

  for (const format of outputFormats.filter(format => format !== 'css')) {
    const fileName = `${baseName}.${format}`
    const output = convertFont(source, format)

    await writeFile(join(outDir, fileName), output)
    cssSources.push({
      fileName,
      format,
    })
  }

  if (outputFormats.includes('css')) {
    if (cssSources.length === 0) {
      throw new Error('build CSS output requires at least one font format')
    }

    await writeFile(
      join(outDir, `${baseName}.css`),
      generateFontFaceCss(cssSources, {
        fontFamily: fontFamily ?? baseName,
        fontPath,
      }),
    )
  }
}

async function buildConfigCommand(configPath) {
  const resolvedConfigPath = resolve(configPath)
  const config = await readConfig(resolvedConfigPath)
  const cwd =
    typeof config.cwd === 'string'
      ? resolve(config.cwd)
      : dirname(resolvedConfigPath)
  const inputs = config.input ?? []
  const outDir = resolve(cwd, config.outDir ?? 'build')
  const outputFormats = outputFormatsFromConfig(config.outputs)

  if (inputs.length === 0) {
    throw new Error('build config requires at least one input')
  }

  if (config.clean === true) {
    await rm(outDir, { recursive: true, force: true })
  }

  await mkdir(outDir, { recursive: true })

  for (const input of inputs) {
    if (typeof input !== 'string') {
      throw new TypeError('build config input entries must be file paths')
    }

    await buildConfigInput(resolve(cwd, input), outDir, outputFormats, config)
  }
}

async function buildConfigInput(input, outDir, outputFormats, config) {
  const contents = await readFile(input)
  const subset = config.subset ?? {}
  const source =
    subset.text === undefined && subset.basicText !== true
      ? contents
      : subsetTtf(contents, {
          basicText: subset.basicText,
          text: subset.text,
        })
  const baseName = basename(input, extname(input))
  const cssSources = []

  for (const format of outputFormats.filter(format => format !== 'css')) {
    const fileName = `${baseName}.${format}`
    const output = convertFont(source, format)

    await writeFile(join(outDir, fileName), output)
    cssSources.push({
      fileName,
      format,
    })
  }

  if (outputFormats.includes('css')) {
    if (cssSources.length === 0) {
      throw new Error('build CSS output requires at least one font format')
    }

    const css = config.css ?? {}

    await writeFile(
      join(outDir, `${baseName}.css`),
      generateFontFaceCss(cssSources, {
        fontDisplay: css.fontDisplay,
        fontFamily: css.fontFamily ?? baseName,
        fontPath: css.fontPath ?? './',
        local: css.local,
      }),
    )
  }
}

async function inspectCommand(args) {
  const json = readFlag(args, ['--json'])
  const [input] = args

  requireValue(input, 'inspect requires an input font')

  const contents = await readFile(input)
  const info = inspectFont(contents)

  if (json) {
    console.log(JSON.stringify(info, undefined, 2))
  } else {
    console.log(
      `${input}: ${info.format}, ${info.size} bytes, ${info.metadata.glyphCount} glyphs`,
    )
  }
}

async function readConfig(configPath) {
  const contents = await readFile(configPath, 'utf8')
  const extension = extname(configPath)

  if (extension === '.json') {
    return JSON.parse(contents)
  }

  if (extension === '.jsonc') {
    return parseJsonc(contents)
  }

  throw new Error(`unsupported config extension \`${extension}\``)
}

async function findConfig(cwd = process.cwd()) {
  for (const fileName of DEFAULT_CONFIG_FILES) {
    const configPath = resolve(cwd, fileName)

    if (await isFile(configPath)) {
      return configPath
    }
  }

  throw new Error(`could not find fontmin config in ${cwd}`)
}

async function isFile(path) {
  try {
    return (await stat(path)).isFile()
  } catch {
    return false
  }
}

function outputFormatsFromConfig(outputs) {
  if (outputs === undefined) {
    return ['eot', 'woff', 'woff2', 'svg', 'css']
  }

  const formats = outputs.map(output =>
    typeof output === 'string' ? output : output.format,
  )

  return parseFormats(formats.join(','))
}

function parseFormats(value) {
  const formats = value
    .split(',')
    .map(format => format.trim().toLowerCase())
    .filter(format => format.length > 0)

  if (formats.length === 0) {
    throw new Error('expected at least one output format')
  }

  for (const format of formats) {
    if (
      format !== 'ttf' &&
      format !== 'woff' &&
      format !== 'woff2' &&
      format !== 'eot' &&
      format !== 'svg' &&
      format !== 'css'
    ) {
      throw new Error(`unsupported output format \`${format}\``)
    }
  }

  return formats
}

function convertFont(contents, format) {
  const normalized = format.toLowerCase()

  if (normalized === 'ttf') {
    if (isTtf(contents)) {
      return contents
    }

    if (isWoff(contents)) {
      return woffToTtf(contents)
    }

    if (isEot(contents)) {
      return eotToTtf(contents)
    }

    throw new Error('unsupported input format for TTF conversion')
  }

  if (normalized === 'woff') {
    return ttfToWoff(contents)
  }

  if (normalized === 'woff2') {
    return ttfToWoff2(contents)
  }

  if (normalized === 'eot') {
    return ttfToEot(contents)
  }

  if (normalized === 'svg') {
    return ttfToSvg(contents)
  }

  throw new Error(`unsupported output format \`${format}\``)
}

function isTtf(contents) {
  return (
    (contents[0] === 0x00 &&
      contents[1] === 0x01 &&
      contents[2] === 0x00 &&
      contents[3] === 0x00) ||
    contents.subarray(0, 4).toString('ascii') === 'true'
  )
}

function isWoff(contents) {
  return contents.subarray(0, 4).toString('ascii') === 'wOFF'
}

function isEot(contents) {
  return (
    contents.byteLength >= 12 &&
    ((contents[8] === 0x01 &&
      contents[9] === 0x00 &&
      contents[10] === 0x02 &&
      contents[11] === 0x00) ||
      (contents[8] === 0x02 &&
        contents[9] === 0x00 &&
        contents[10] === 0x02 &&
        contents[11] === 0x00))
  )
}

function readOption(args, names) {
  const index = args.findIndex(arg => names.includes(arg))

  if (index === -1) {
    return
  }

  const [name] = args.splice(index, 1)
  const [value] = args.splice(index, 1)

  if (value === undefined || value.startsWith('-')) {
    throw new Error(`${name} requires a value`)
  }

  return value
}

function readFlag(args, names) {
  const index = args.findIndex(arg => names.includes(arg))

  if (index === -1) {
    return false
  }

  args.splice(index, 1)
  return true
}

function requireValue(value, message) {
  if (value === undefined || value.length === 0) {
    throw new Error(message)
  }
}

async function writeOutput(output, contents) {
  await mkdir(dirname(output), { recursive: true })
  await writeFile(output, contents)
}

function usage() {
  console.error(`Usage:
  fontmin-rs subset <input.ttf> -o <output.ttf> --text <text>
  fontmin-rs convert <input.ttf> -f <ttf|woff|woff2|eot|svg> -o <output>
  fontmin-rs build <input.ttf> -o <out-dir> --formats <ttf,woff,woff2,eot,svg,css> [--text <text>]
  fontmin-rs inspect <input.ttf> [--json]`)
}
