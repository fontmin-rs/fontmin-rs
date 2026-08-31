import { createHash } from 'node:crypto'
import { mkdir, readFile, rm } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import {
  builtinPluginDescriptor,
  internalCacheKey,
  isCacheablePlugin,
  pluginUsesRuntime,
} from './builtin-plugin'
import { withCacheLock } from './cache-lock'
import type { OptimizeRuntime, RuntimeSelector } from './optimize-runtime'
import type {
  ArtifactFormat,
  CacheOptions,
  FontAsset,
  FontFormat,
  FontminConfig,
  FontminPlugin,
  PluginContext,
} from './types'
import {
  atomicWriteFile,
  ensureRealPathContained,
  resolveContainedPath,
} from './workspace-io'

export interface NormalizedCacheOptions {
  dir: string
  enabled: boolean
  maxAgeMs: number
  maxEntries: number
}

interface CacheAssetRecord {
  fileName: string
  format: ArtifactFormat
  meta: Record<string, unknown>
  path: string
  sha256: string
  size: number
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
  entries: Record<string, CacheIndexEntry>
  version: string
}

interface CacheIndexEntry {
  assets: string[]
  updatedAt: string
}

const CACHE_SCHEMA_VERSION = 'v1'
const FONTMIN_VERSION = '1.2.0-rc.1'
const DEFAULT_CACHE_DIR = 'node_modules/.cache/fontmin-rs'
const DEFAULT_CACHE_MAX_AGE_MS = 30 * 24 * 60 * 60 * 1000
const DEFAULT_CACHE_MAX_ENTRIES = 256
const CACHE_KEY_PATTERN = /^[\da-f]{64}$/u
// Cache corruption becomes a validated miss, so atomic replacement is enough;
// forcing each manifest and index update to stable storage makes scale linear
// in fsync latency.
const CACHE_WRITE_OPTIONS = { flush: false } as const
const ARTIFACT_FORMATS = new Set<ArtifactFormat>([
  'css',
  'eot',
  'html',
  'json',
  'otf',
  'svg',
  'ttf',
  'unknown',
  'woff',
  'woff2',
])
const FONT_FORMATS = new Set<FontFormat>([
  'css',
  'eot',
  'otf',
  'svg',
  'ttf',
  'unknown',
  'woff',
  'woff2',
])

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
      await atomicWriteFile(filePath, contents)
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
  let value: unknown

  try {
    value = JSON.parse(await readFile(cacheManifestPath(cacheDir, key), 'utf8'))
  } catch {
    return undefined
  }

  if (!isCacheManifest(value)) {
    return undefined
  }
  const manifest = value
  if (
    manifest.version !== CACHE_SCHEMA_VERSION ||
    manifest.key !== key ||
    manifest.runtime.requested !== runtime.requested ||
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
      if (
        contents.byteLength !== record.size ||
        sha256(contents) !== record.sha256
      ) {
        return undefined
      }

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
  cache: NormalizedCacheOptions,
  key: string,
  runtime: CacheRuntimeIdentity,
  assets: FontAsset[],
): Promise<void> {
  const cacheDir = cache.dir

  await withCacheLock(cacheRoot(cacheDir), async () => {
    const entryDir = cacheEntryDir(cacheDir, key)
    const records: CacheAssetRecord[] = []

    await mkdir(entryDir, { recursive: true })

    for (const [index, asset] of assets.entries()) {
      const fileName = `${String(index).padStart(3, '0')}.${asset.format}`

      await atomicWriteFile(
        join(entryDir, fileName),
        asset.contents,
        CACHE_WRITE_OPTIONS,
      )
      records.push({
        fileName,
        format: asset.format,
        meta: asset.meta,
        path: asset.path,
        sha256: sha256(asset.contents),
        size: asset.contents.byteLength,
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
      CACHE_WRITE_OPTIONS,
    )
    await updateCacheIndex(
      cacheDir,
      key,
      records,
      cache.maxEntries,
      cache.maxAgeMs,
    )
  })
}

async function updateCacheIndex(
  cacheDir: string,
  key: string,
  assets: CacheAssetRecord[],
  maxEntries: number,
  maxAgeMs: number,
): Promise<void> {
  const indexPath = cacheIndexPath(cacheDir)
  let index: CacheIndex = {
    entries: {},
    version: CACHE_SCHEMA_VERSION,
  }

  try {
    index = normalizeCacheIndex(JSON.parse(await readFile(indexPath, 'utf8')))
  } catch {
    // A missing or corrupted cache index can be rebuilt from the next writes.
  }

  index.entries[key] = {
    assets: assets.map(asset => asset.path),
    updatedAt: new Date().toISOString(),
  }

  const retainedEntries = Object.entries(index.entries)
    .filter(
      ([entryKey, entry]) =>
        entryKey === key || cacheEntryTimestamp(entry) >= Date.now() - maxAgeMs,
    )
    .toSorted(([leftKey, left], [rightKey, right]) => {
      if (leftKey === key) {
        return -1
      }
      if (rightKey === key) {
        return 1
      }

      return (
        cacheEntryTimestamp(right) - cacheEntryTimestamp(left) ||
        leftKey.localeCompare(rightKey)
      )
    })
    .slice(0, maxEntries)
  const retainedKeys = new Set(retainedEntries.map(([entryKey]) => entryKey))
  const prunedKeys = Object.keys(index.entries).filter(
    entryKey => !retainedKeys.has(entryKey),
  )

  index.entries = Object.fromEntries(retainedEntries)

  for (const entryKey of prunedKeys) {
    await rm(cacheEntryDir(cacheDir, entryKey), {
      force: true,
      recursive: true,
    })
  }

  await mkdir(dirname(indexPath), { recursive: true })
  await atomicWriteFile(
    indexPath,
    `${JSON.stringify(index, undefined, 2)}\n`,
    CACHE_WRITE_OPTIONS,
  )
}

function normalizeCacheIndex(value: unknown): CacheIndex {
  const empty: CacheIndex = {
    entries: {},
    version: CACHE_SCHEMA_VERSION,
  }

  if (
    !isRecord(value) ||
    value['version'] !== CACHE_SCHEMA_VERSION ||
    !isRecord(value['entries'])
  ) {
    return empty
  }

  for (const [key, entry] of Object.entries(value['entries'])) {
    if (
      !isCacheKey(key) ||
      !isRecord(entry) ||
      !Array.isArray(entry['assets']) ||
      !entry['assets'].every(asset => typeof asset === 'string') ||
      typeof entry['updatedAt'] !== 'string'
    ) {
      continue
    }

    empty.entries[key] = {
      assets: entry['assets'],
      updatedAt: entry['updatedAt'],
    }
  }

  return empty
}

function isCacheManifest(value: unknown): value is CacheManifest {
  return (
    isRecord(value) &&
    typeof value['key'] === 'string' &&
    value['version'] === CACHE_SCHEMA_VERSION &&
    isRecord(value['runtime']) &&
    typeof value['runtime']['requested'] === 'string' &&
    (typeof value['runtime']['resolved'] === 'string' ||
      value['runtime']['resolved'] === null) &&
    Array.isArray(value['assets']) &&
    value['assets'].every(isCacheAssetRecord)
  )
}

function isCacheAssetRecord(value: unknown): value is CacheAssetRecord {
  return (
    isRecord(value) &&
    typeof value['fileName'] === 'string' &&
    typeof value['format'] === 'string' &&
    ARTIFACT_FORMATS.has(value['format'] as ArtifactFormat) &&
    isRecord(value['meta']) &&
    typeof value['path'] === 'string' &&
    typeof value['sha256'] === 'string' &&
    CACHE_KEY_PATTERN.test(value['sha256']) &&
    Number.isSafeInteger(value['size']) &&
    (value['size'] as number) >= 0 &&
    typeof value['sourceFormat'] === 'string' &&
    FONT_FORMATS.has(value['sourceFormat'] as FontFormat)
  )
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function isCacheKey(value: string): boolean {
  return CACHE_KEY_PATTERN.test(value)
}

function cacheEntryTimestamp(entry: CacheIndexEntry): number {
  const timestamp = Date.parse(entry.updatedAt)

  return Number.isFinite(timestamp) ? timestamp : Number.NEGATIVE_INFINITY
}

function cacheEntryDir(cacheDir: string, key: string): string {
  if (!isCacheKey(key)) {
    throw new TypeError('cache key must be a lowercase SHA-256 digest')
  }

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
        native: builtinPluginDescriptor(plugin),
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
    plugins.some(plugin => pluginUsesRuntime(plugin))
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
  const objectOptions = typeof options === 'object' ? options : undefined
  const configuredDir =
    objectOptions?.dir === undefined ? DEFAULT_CACHE_DIR : objectOptions.dir
  const maxAgeMs = objectOptions?.maxAgeMs ?? DEFAULT_CACHE_MAX_AGE_MS
  const maxEntries = objectOptions?.maxEntries ?? DEFAULT_CACHE_MAX_ENTRIES

  if (!Number.isSafeInteger(maxAgeMs) || maxAgeMs < 1) {
    throw new TypeError('cache maxAgeMs must be a positive integer')
  }
  if (!Number.isSafeInteger(maxEntries) || maxEntries < 1) {
    throw new TypeError('cache maxEntries must be a positive integer')
  }

  const normalized = {
    dir: resolve(cwd, configuredDir),
    maxAgeMs,
    maxEntries,
  }

  if (override === true) {
    return {
      ...normalized,
      enabled: true,
    }
  }

  if (override === false || options === undefined || options === false) {
    return {
      ...normalized,
      enabled: false,
    }
  }

  if (options === true) {
    return {
      ...normalized,
      enabled: true,
    }
  }

  return {
    ...normalized,
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

export function isCacheablePipeline(plugins: FontminPlugin[]): boolean {
  return plugins.every(plugin => isCacheablePlugin(plugin))
}
