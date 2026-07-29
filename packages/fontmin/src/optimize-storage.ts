import { createHash } from 'node:crypto'
import { mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { withCacheLock } from './cache-lock'
import type { OptimizeRuntime, RuntimeSelector } from './optimize-runtime'
import { internalCacheKey } from './optimize-transforms'
import type {
  AssetFormat,
  CacheOptions,
  FontAsset,
  FontFormat,
  FontminConfig,
  FontminPlugin,
  PluginContext,
} from './types'
import { ensureRealPathContained, resolveContainedPath } from './workspace-io'

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
  override?: boolean,
): NormalizedCacheOptions {
  const configuredDir =
    typeof options === 'object' && options.dir !== undefined
      ? options.dir
      : DEFAULT_CACHE_DIR

  if (override === true) {
    return {
      dir: resolve(cwd, configuredDir),
      enabled: true,
    }
  }

  if (override === false || options === undefined || options === false) {
    return {
      dir: resolve(cwd, configuredDir),
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
