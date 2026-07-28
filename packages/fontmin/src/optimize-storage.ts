import { createHash } from 'node:crypto'
import {
  lstat,
  mkdir,
  readFile,
  realpath,
  rename,
  rm,
  writeFile,
} from 'node:fs/promises'
import {
  basename,
  dirname,
  isAbsolute,
  join,
  parse,
  relative,
  resolve,
  sep,
} from 'node:path'
import { glob } from 'tinyglobby'
import { withCacheLock } from './cache-lock'
import type { OptimizeRuntime, RuntimeSelector } from './optimize-runtime'
import {
  detectFormat,
  extensionForFormat,
  internalCacheKey,
  isBuiltin,
} from './optimize-transforms'
import type {
  AssetFormat,
  CacheOptions,
  FontAsset,
  FontFormat,
  FontminConfig,
  FontminPlugin,
  PluginContext,
  SubsetOptions,
} from './types'

export interface NormalizedCacheOptions {
  dir: string
  enabled: boolean
}

interface CacheAssetRecord {
  fileName: string
  format: AssetFormat
  meta: Record<string, unknown>
  path: string
  sourceFormat: FontFormat
}

interface CacheManifest {
  assets: CacheAssetRecord[]
  key: string
  runtime: CacheRuntimeIdentity
  version: string
}

export interface CacheRuntimeIdentity {
  requested: RuntimeSelector['requested']
  resolved: OptimizeRuntime['kind'] | null
}

interface CacheIndex {
  entries: Record<
    string,
    {
      assets: string[]
      updatedAt: string
    }
  >
  version: string
}

const CACHE_SCHEMA_VERSION = 'v1'
const FONTMIN_VERSION = '1.0.0'
const DEFAULT_CACHE_DIR = 'node_modules/.cache/fontmin-rs'
let temporaryFileCounter = 0

export function createPluginContext(
  cwd: string,
  emittedAssets: FontAsset[],
): PluginContext {
  const diagnostics: PluginContext['diagnostics'] = []

  return {
    cwd,
    diagnostics,
    emitFile(asset) {
      emittedAssets.push(asset)
    },
    readFile(path) {
      return readFile(resolve(cwd, path))
    },
    resolve(path) {
      return resolve(cwd, path)
    },
    warn(message) {
      diagnostics.push({
        level: 'warn',
        message: warningMessage(message),
      })
    },
    async writeFile(path, contents) {
      const filePath = resolve(cwd, path)

      await mkdir(dirname(filePath), { recursive: true })
      await writeFile(filePath, contents)
    },
  }
}

function warningMessage(message: string | Error): string {
  return typeof message === 'string' ? message : message.message
}

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
  if (!isBuiltin(plugin, 'glyph')) {
    return plugin
  }

  const options = await resolveSubsetTextFile(
    plugin.native.options as SubsetOptions,
    cwd,
  )

  if (options === plugin.native.options) {
    return plugin
  }

  return {
    ...plugin,
    native: {
      ...plugin.native,
      options: options as Record<string, unknown>,
    },
  }
}

async function resolveSubsetTextFile(
  options: SubsetOptions,
  cwd: string,
): Promise<SubsetOptions> {
  if (options.textFile === undefined) {
    return options
  }

  const fileText = await readFile(resolve(cwd, options.textFile), 'utf8')
  const { textFile: _textFile, ...resolvedOptions } = options

  return {
    ...resolvedOptions,
    text: mergeSubsetText(options.text, fileText),
  }
}

function mergeSubsetText(text: string | undefined, fileText: string): string {
  return text === undefined ? fileText : `${text}${fileText}`
}

export async function readCachedAssets(
  cacheDir: string,
  key: string,
  runtime: CacheRuntimeIdentity,
): Promise<FontAsset[] | undefined> {
  let manifest: CacheManifest

  try {
    manifest = JSON.parse(
      await readFile(cacheManifestPath(cacheDir, key), 'utf8'),
    ) as CacheManifest
  } catch {
    return undefined
  }

  if (
    manifest.version !== CACHE_SCHEMA_VERSION ||
    manifest.key !== key ||
    manifest.runtime?.requested !== runtime.requested ||
    manifest.runtime.resolved !== runtime.resolved
  ) {
    return undefined
  }

  const entryDir = cacheEntryDir(cacheDir, key)
  const assets: FontAsset[] = []

  try {
    for (const record of manifest.assets) {
      const cacheFile = resolveContainedPath(
        entryDir,
        record.fileName,
        'cache file name',
      )

      await ensureRealPathContained(entryDir, cacheFile, 'cache file name')
      const contents = await readFile(cacheFile)

      assets.push({
        path: record.path,
        contents,
        format: record.format,
        sourceFormat: record.sourceFormat,
        meta: {
          ...record.meta,
          cache: {
            hit: true,
            key,
          },
        },
      })
    }
  } catch {
    return undefined
  }

  return assets
}

export async function writeCachedAssets(
  cacheDir: string,
  key: string,
  runtime: CacheRuntimeIdentity,
  assets: FontAsset[],
): Promise<void> {
  await withCacheLock(cacheRoot(cacheDir), async () => {
    const entryDir = cacheEntryDir(cacheDir, key)
    const records: CacheAssetRecord[] = []

    await mkdir(entryDir, { recursive: true })

    for (const [index, asset] of assets.entries()) {
      const fileName = `${String(index).padStart(3, '0')}.${asset.format}`

      await atomicWriteFile(join(entryDir, fileName), asset.contents)
      records.push({
        fileName,
        format: asset.format,
        meta: asset.meta,
        path: asset.path,
        sourceFormat: asset.sourceFormat,
      })
    }

    await atomicWriteFile(
      cacheManifestPath(cacheDir, key),
      `${JSON.stringify(
        {
          assets: records,
          key,
          runtime,
          version: CACHE_SCHEMA_VERSION,
        } satisfies CacheManifest,
        undefined,
        2,
      )}\n`,
    )
    await updateCacheIndex(cacheDir, key, records)
  })
}

async function updateCacheIndex(
  cacheDir: string,
  key: string,
  assets: CacheAssetRecord[],
): Promise<void> {
  const indexPath = cacheIndexPath(cacheDir)
  let index: CacheIndex = {
    entries: {},
    version: CACHE_SCHEMA_VERSION,
  }

  try {
    index = JSON.parse(await readFile(indexPath, 'utf8')) as CacheIndex
  } catch {
    // A missing or corrupted cache index can be rebuilt from the next writes.
  }

  if (index.version !== CACHE_SCHEMA_VERSION) {
    index = {
      entries: {},
      version: CACHE_SCHEMA_VERSION,
    }
  }

  index.entries[key] = {
    assets: assets.map(asset => asset.path),
    updatedAt: new Date().toISOString(),
  }

  await mkdir(dirname(indexPath), { recursive: true })
  await atomicWriteFile(indexPath, `${JSON.stringify(index, undefined, 2)}\n`)
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

async function expandInputPath(input: string, cwd: string): Promise<string[]> {
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

function cacheEntryDir(cacheDir: string, key: string): string {
  return join(cacheRoot(cacheDir), key.slice(0, 2), key.slice(2, 4), key)
}

function cacheIndexPath(cacheDir: string): string {
  return join(cacheRoot(cacheDir), 'index.json')
}

function cacheManifestPath(cacheDir: string, key: string): string {
  return join(cacheEntryDir(cacheDir, key), 'index.json')
}

function cacheRoot(cacheDir: string): string {
  return join(cacheDir, CACHE_SCHEMA_VERSION)
}

export function cacheKeyForAssets(
  assets: FontAsset[],
  config: FontminConfig,
  plugins: FontminPlugin[],
  runtime: CacheRuntimeIdentity,
): string {
  return sha256(
    stableStringify({
      clean: config.clean,
      fontminVersion: FONTMIN_VERSION,
      inputs: assets.map(asset => ({
        format: asset.format,
        hash: sha256(asset.contents),
        path: asset.path,
        sourceFormat: asset.sourceFormat,
      })),
      plugins: plugins.map(plugin => ({
        enforce: plugin.enforce,
        internalCacheKey: internalCacheKey(plugin),
        name: plugin.name,
        native: plugin.native,
      })),
      preserveOriginal: config.preserveOriginal,
      runtime,
      schema: CACHE_SCHEMA_VERSION,
      subset: config.subset,
    }),
  )
}

export async function cacheRuntimeIdentity(
  config: FontminConfig,
  plugins: FontminPlugin[],
  runtime: RuntimeSelector,
): Promise<CacheRuntimeIdentity> {
  const usesRuntime =
    config.subset !== undefined ||
    plugins.some(plugin => plugin.native?.kind === 'builtin')
  const resolved = usesRuntime ? await runtime.resolve() : undefined

  return {
    requested: runtime.requested,
    resolved: resolved?.kind ?? null,
  }
}

export function normalizeCacheOptions(
  options: boolean | CacheOptions | undefined,
  cwd: string,
): NormalizedCacheOptions {
  if (options === undefined || options === false) {
    return {
      dir: resolve(cwd, DEFAULT_CACHE_DIR),
      enabled: false,
    }
  }

  if (options === true) {
    return {
      dir: resolve(cwd, DEFAULT_CACHE_DIR),
      enabled: true,
    }
  }

  return {
    dir: resolve(cwd, options.dir ?? DEFAULT_CACHE_DIR),
    enabled: options.enabled ?? true,
  }
}

function sha256(input: string | Uint8Array): string {
  return createHash('sha256').update(input).digest('hex')
}

function stableStringify(value: unknown): string {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value)
  }

  if (Array.isArray(value)) {
    return `[${value.map(item => stableStringify(item)).join(',')}]`
  }

  const entries = Object.entries(value)
    .filter(([, entryValue]) => entryValue !== undefined)
    .sort(([left], [right]) => left.localeCompare(right))

  return `{${entries
    .map(([key, entryValue]) => {
      return `${JSON.stringify(key)}:${stableStringify(entryValue)}`
    })
    .join(',')}}`
}

export async function writeAssets(
  outDir: string,
  assets: FontAsset[],
): Promise<void> {
  for (const asset of assets) {
    const outputPath = resolveContainedPath(outDir, asset.path, 'asset path')

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

function resolveContainedPath(
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

async function ensureRealPathContained(
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

async function atomicWriteFile(
  path: string,
  contents: string | Uint8Array,
): Promise<void> {
  const temporaryPath = `${path}.${process.pid}.${temporaryFileCounter}.tmp`
  temporaryFileCounter += 1

  try {
    await writeFile(temporaryPath, contents)
    await rename(temporaryPath, path)
  } finally {
    await rm(temporaryPath, { force: true })
  }
}

function isMissingFileError(error: unknown): boolean {
  return isNodeErrorWithCode(error, 'ENOENT')
}

function isNodeErrorWithCode(error: unknown, code: string): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    error.code === code
  )
}

export function isCacheablePipeline(plugins: FontminPlugin[]): boolean {
  return plugins.every(plugin => {
    if (internalCacheKey(plugin) !== undefined) {
      return true
    }

    return (
      plugin.native?.kind === 'builtin' &&
      plugin.buildStart === undefined &&
      plugin.transform === undefined &&
      plugin.generateBundle === undefined &&
      plugin.buildEnd === undefined
    )
  })
}
