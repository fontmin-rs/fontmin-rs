/**
 * Node-only workspace boundary for resolving inputs and text files, cleaning
 * output directories, and writing assets without escaping configured roots.
 */
import {
  lstat,
  mkdir,
  readFile,
  realpath,
  rm,
  writeFile,
} from 'node:fs/promises'
import {
  basename,
  dirname,
  isAbsolute,
  parse,
  relative,
  resolve,
  sep,
} from 'node:path'
import { glob } from 'tinyglobby'
import {
  builtinPluginDescriptor,
  withBuiltinPluginOptions,
} from './builtin-plugin'
import { detectFormat, extensionForFormat } from './optimize-transforms'
import type {
  FontAsset,
  FontminConfig,
  FontminPlugin,
  SubsetOptions,
} from './types'
import { discoverWebText } from './web-text'

export async function resolveConfigTextFile(
  config: FontminConfig,
  cwd: string,
): Promise<FontminConfig> {
  if (config.subset === undefined) {
    return config
  }

  const subset = await resolveSubsetTextFile(config.subset, cwd)

  if (subset === config.subset) {
    return config
  }

  return {
    ...config,
    subset,
  }
}

export async function resolvePluginTextFiles(
  plugins: FontminPlugin[],
  cwd: string,
): Promise<FontminPlugin[]> {
  const resolvedPlugins: FontminPlugin[] = []

  for (const plugin of plugins) {
    resolvedPlugins.push(await resolvePluginTextFile(plugin, cwd))
  }

  return resolvedPlugins
}

async function resolvePluginTextFile(
  plugin: FontminPlugin,
  cwd: string,
): Promise<FontminPlugin> {
  const descriptor = builtinPluginDescriptor(plugin, 'glyph')

  if (descriptor === undefined) {
    return plugin
  }

  const options = await resolveSubsetTextFile(
    descriptor.options as SubsetOptions,
    cwd,
  )

  if (options === descriptor.options) {
    return plugin
  }

  return withBuiltinPluginOptions(
    plugin,
    'glyph',
    options as Record<string, unknown>,
  )
}

export async function resolveSubsetTextFile(
  options: SubsetOptions,
  cwd: string,
): Promise<SubsetOptions> {
  if (options.textFile === undefined && options.content === undefined) {
    return options
  }

  const fileText =
    options.textFile === undefined
      ? ''
      : await readFile(resolve(cwd, options.textFile), 'utf8')
  const discovery =
    options.content === undefined
      ? undefined
      : await discoverWebText({ cwd, files: options.content })
  const discoveredText = discovery?.text ?? ''
  const { content: _content, textFile: _textFile, ...resolvedOptions } = options

  return {
    ...resolvedOptions,
    text: mergeSubsetText(options.text, `${fileText}${discoveredText}`),
  }
}

function mergeSubsetText(text: string | undefined, fileText: string): string {
  return text === undefined ? fileText : `${text}${fileText}`
}

export async function loadInputAssets(
  inputs: (string | Uint8Array)[],
  cwd: string,
): Promise<FontAsset[]> {
  if (inputs.length === 0) {
    throw new Error('fontmin-rs optimize requires at least one input')
  }

  const assets: FontAsset[] = []

  for (const input of inputs) {
    if (typeof input === 'string') {
      const inputPaths = await expandInputPath(input, cwd)

      for (const inputPath of inputPaths) {
        const contents = await readFile(inputPath)
        const format = detectFormat(contents)

        assets.push({
          path: basename(inputPath),
          contents,
          format,
          sourceFormat: format,
          meta: { inputPath },
        })
      }
    } else {
      const contents = Buffer.from(input)
      const format = detectFormat(contents)

      assets.push({
        path: `fontmin.${extensionForFormat(format)}`,
        contents,
        format,
        sourceFormat: format,
        meta: {},
      })
    }
  }

  return assets
}

export async function expandInputPath(
  input: string,
  cwd: string,
): Promise<string[]> {
  if (!isGlobPattern(input)) {
    return [resolve(cwd, input)]
  }

  const matches = await glob(input, {
    absolute: true,
    cwd,
    onlyFiles: true,
  })

  if (matches.length === 0) {
    throw new Error(`fontmin-rs input glob matched no files: ${input}`)
  }

  return matches.sort((left, right) => left.localeCompare(right))
}

function isGlobPattern(path: string): boolean {
  return /[*?[\]{}]/u.test(path)
}

export async function writeAssets(
  outDir: string,
  assets: FontAsset[],
): Promise<void> {
  const outputPaths = new Set<string>()
  const outputs = assets.map(asset => {
    const outputPath = resolveContainedPath(outDir, asset.path, 'asset path')

    if (outputPaths.has(outputPath)) {
      throw new Error(`duplicate output path: ${asset.path}`)
    }
    outputPaths.add(outputPath)

    return { asset, outputPath }
  })

  for (const { asset, outputPath } of outputs) {
    await mkdir(dirname(outputPath), { recursive: true })
    await ensureRealPathContained(outDir, dirname(outputPath), 'asset path')
    await rejectSymbolicLink(outputPath)
    await writeFile(outputPath, asset.contents)
  }
}

export async function cleanOutputDirectory(
  cwd: string,
  outDir: string,
  protectedPaths: string[],
): Promise<void> {
  const root = resolve(cwd)
  const target = resolve(outDir)
  const targetIsInsideRoot = pathContains(root, target) && target !== root
  const targetContainsInput = protectedPaths.some(path =>
    pathContains(target, resolve(path)),
  )

  if (
    target === parse(target).root ||
    pathContains(target, root) ||
    targetContainsInput
  ) {
    throw new Error(
      `refusing to clean output directory ${target} because it is the project directory, an input ancestor, or a filesystem root`,
    )
  }

  try {
    const [realRoot, realTarget, ...realProtectedPaths] = await Promise.all([
      realpath(root),
      realpath(target),
      ...protectedPaths.map(path => realpath(resolve(path))),
    ])

    if (
      realTarget === parse(realTarget).root ||
      pathContains(realTarget, realRoot) ||
      realProtectedPaths.some(path => pathContains(realTarget, path)) ||
      (targetIsInsideRoot && !pathContains(realRoot, realTarget))
    ) {
      throw new Error(
        `refusing to clean output directory ${target} because its resolved location is unsafe for project ${root}`,
      )
    }
  } catch (error) {
    if (!isMissingFileError(error)) {
      throw error
    }
  }

  await rm(target, { recursive: true, force: true })
}

function pathContains(parent: string, candidate: string): boolean {
  const childPath = relative(parent, candidate)

  return (
    childPath === '' ||
    (childPath !== '..' &&
      !childPath.startsWith(`..${sep}`) &&
      !isAbsolute(childPath))
  )
}

export function resolveContainedPath(
  root: string,
  path: string,
  label: string,
): string {
  if (path.length === 0 || isAbsolute(path)) {
    throw new Error(`${label} must be a non-empty relative path: ${path}`)
  }

  const resolvedRoot = resolve(root)
  const resolvedPath = resolve(resolvedRoot, path)
  const relativePath = relative(resolvedRoot, resolvedPath)

  if (
    relativePath === '' ||
    relativePath === '..' ||
    relativePath.startsWith(`..${sep}`) ||
    isAbsolute(relativePath)
  ) {
    throw new Error(
      `${label} must stay within its destination directory: ${path}`,
    )
  }

  return resolvedPath
}

export async function ensureRealPathContained(
  root: string,
  path: string,
  label: string,
): Promise<void> {
  const [realRoot, realPath] = await Promise.all([
    realpath(root),
    realpath(path),
  ])
  const relativePath = relative(realRoot, realPath)

  if (
    relativePath === '..' ||
    relativePath.startsWith(`..${sep}`) ||
    isAbsolute(relativePath)
  ) {
    throw new Error(`${label} resolves outside its destination directory`)
  }
}

async function rejectSymbolicLink(path: string): Promise<void> {
  try {
    const metadata = await lstat(path)

    if (metadata.isSymbolicLink()) {
      throw new Error(`refusing to write output through symbolic link: ${path}`)
    }
  } catch (error) {
    if (!isMissingFileError(error)) {
      throw error
    }
  }
}

function isMissingFileError(error: unknown): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    error.code === 'ENOENT'
  )
}
