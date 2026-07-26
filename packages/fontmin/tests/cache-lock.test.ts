import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import { withCacheLock as withBinCacheLock } from '../bin/cache-lock.mjs'
import { withCacheLock } from '../src/cache-lock'

interface Deferred {
  promise: Promise<void>
  resolve: () => void
}

type CacheLock = (
  cacheDir: string,
  operation: () => Promise<void>,
) => Promise<void>

const implementations: {
  name: string
  withCacheLock: CacheLock
}[] = [
  { name: 'Node API', withCacheLock },
  { name: 'package CLI', withCacheLock: withBinCacheLock },
]

function createDeferred(): Deferred {
  let resolvePromise: (() => void) | undefined
  const promise = new Promise<void>(resolve => {
    resolvePromise = resolve
  })

  return {
    promise,
    resolve() {
      resolvePromise?.()
    },
  }
}

it.each(implementations)(
  '$name keeps a replacement cache lock owned by another writer',
  async implementation => {
    const cacheDir = await mkdtemp(
      resolve(tmpdir(), 'fontmin-rs-cache-lock-owner-'),
    )
    const cacheRoot = resolve(cacheDir, 'v1')
    const lockPath = resolve(cacheRoot, '.write.lock')
    const acquired = createDeferred()
    const release = createDeferred()

    try {
      const operation = implementation.withCacheLock(cacheRoot, async () => {
        acquired.resolve()
        await release.promise
      })

      await acquired.promise
      await rm(lockPath)
      await writeFile(lockPath, 'successor')
      release.resolve()
      await operation

      await expect(readFile(lockPath, 'utf8')).resolves.toBe('successor')
    } finally {
      await mkdir(cacheDir, { recursive: true })
      await rm(cacheDir, { force: true, recursive: true })
    }
  },
)
