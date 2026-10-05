import {
  mkdir,
  mkdtemp,
  readFile,
  rm,
  utimes,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { relative, resolve } from 'node:path'
import { expect, it, vi } from 'vitest'
import { withCacheLock } from '../src/cache-lock'

interface Deferred {
  promise: Promise<void>
  resolve: () => void
}

type CacheLock = <T>(
  cacheDir: string,
  operation: () => Promise<T>,
) => Promise<T>

const implementations: {
  name: string
  withCacheLock: CacheLock
}[] = [{ name: 'Node API', withCacheLock }]

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
  '$name queues writers in call order without polling the same cache directory',
  async implementation => {
    const cacheDir = await mkdtemp(
      resolve(tmpdir(), 'fontmin-rs-cache-lock-queue-'),
    )
    const cacheRoot = resolve(cacheDir, 'v1')
    const acquired = createDeferred()
    const release = createDeferred()
    const events: string[] = []
    const firstOperation = implementation.withCacheLock(cacheRoot, async () => {
      events.push('start:0')
      acquired.resolve()
      await release.promise
      events.push('end:0')
      return 0
    })
    const operations = [firstOperation]

    try {
      await acquired.promise
      const retryDelay = vi.spyOn(globalThis, 'setTimeout')

      try {
        const cacheAliases = [
          cacheRoot,
          `${cacheRoot}/../v1`,
          relative(process.cwd(), cacheRoot),
        ]

        for (const [offset, cacheAlias] of cacheAliases.entries()) {
          const index = offset + 1

          operations.push(
            implementation.withCacheLock(cacheAlias, async () => {
              events.push(`start:${index}`)
              await readFile(resolve(cacheRoot, '.write.lock'), 'utf8')
              events.push(`end:${index}`)
              return index
            }),
          )
        }

        release.resolve()

        await expect(Promise.all(operations)).resolves.toStrictEqual(
          Array.from({ length: 4 }, (_, index) => index),
        )
        expect(events).toStrictEqual(
          Array.from({ length: 4 }, (_, index) => [
            `start:${index}`,
            `end:${index}`,
          ]).flat(),
        )
        expect(retryDelay).not.toHaveBeenCalled()
      } finally {
        retryDelay.mockRestore()
      }
    } finally {
      release.resolve()
      await Promise.allSettled(operations)
      await rm(cacheDir, { force: true, recursive: true })
    }
  },
)

it.each(implementations)(
  '$name continues queued and later writes after an operation fails',
  async implementation => {
    const cacheDir = await mkdtemp(
      resolve(tmpdir(), 'fontmin-rs-cache-lock-failure-'),
    )
    const cacheRoot = resolve(cacheDir, 'v1')
    const acquired = createDeferred()
    const release = createDeferred()
    const writeError = new Error('cache write failed')
    const firstOperation = implementation.withCacheLock(cacheRoot, async () => {
      acquired.resolve()
      await release.promise
      throw writeError
    })
    const rejected = (async () => {
      await expect(firstOperation).rejects.toBe(writeError)
    })()
    const operations: Promise<unknown>[] = [firstOperation]

    try {
      await acquired.promise
      const secondOperation = implementation.withCacheLock(
        cacheRoot,
        async () => 'queued write',
      )
      operations.push(secondOperation)
      release.resolve()

      await rejected
      await expect(secondOperation).resolves.toBe('queued write')
      await expect(
        implementation.withCacheLock(cacheRoot, async () => 'later write'),
      ).resolves.toBe('later write')
      await expect(
        readFile(resolve(cacheRoot, '.write.lock'), 'utf8'),
      ).rejects.toMatchObject({ code: 'ENOENT' })
    } finally {
      release.resolve()
      await Promise.allSettled(operations)
      await rm(cacheDir, { force: true, recursive: true })
    }
  },
)

it.each(implementations)(
  '$name allows another write after lock acquisition fails',
  async implementation => {
    const cacheDir = await mkdtemp(
      resolve(tmpdir(), 'fontmin-rs-cache-lock-acquisition-failure-'),
    )
    const cacheRoot = resolve(cacheDir, 'v1')
    const operation = vi.fn<() => Promise<string>>(
      async () => 'recovered write',
    )

    try {
      await writeFile(cacheRoot, 'not a directory')
      await expect(
        implementation.withCacheLock(cacheRoot, operation),
      ).rejects.toMatchObject({ code: 'EEXIST' })
      expect(operation).not.toHaveBeenCalled()

      await rm(cacheRoot)
      await expect(
        implementation.withCacheLock(cacheRoot, operation),
      ).resolves.toBe('recovered write')
      expect(operation).toHaveBeenCalledOnce()
    } finally {
      await rm(cacheDir, { force: true, recursive: true })
    }
  },
)

it.each(implementations)(
  '$name allows different cache directories to write independently',
  async implementation => {
    const cacheDir = await mkdtemp(
      resolve(tmpdir(), 'fontmin-rs-cache-lock-independent-'),
    )
    const acquired = createDeferred()
    const release = createDeferred()
    const firstOperation = implementation.withCacheLock(
      resolve(cacheDir, 'first'),
      async () => {
        acquired.resolve()
        await release.promise
      },
    )

    try {
      await acquired.promise
      await expect(
        implementation.withCacheLock(
          resolve(cacheDir, 'second'),
          async () => 'independent write',
        ),
      ).resolves.toBe('independent write')
    } finally {
      release.resolve()
      await firstOperation
      await rm(cacheDir, { force: true, recursive: true })
    }
  },
)

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

it.each(implementations)(
  '$name does not expire a lock owned by a live process',
  async implementation => {
    const cacheDir = await mkdtemp(
      resolve(tmpdir(), 'fontmin-rs-cache-lock-live-owner-'),
    )
    const cacheRoot = resolve(cacheDir, 'v1')
    const lockPath = resolve(cacheRoot, '.write.lock')
    let acquired = false

    try {
      await mkdir(cacheRoot, { recursive: true })
      await writeFile(lockPath, `${process.pid}:existing-writer`)
      const staleTime = new Date(Date.now() - 10 * 60_000)
      await utimes(lockPath, staleTime, staleTime)
      const operation = implementation.withCacheLock(cacheRoot, async () => {
        acquired = true
      })

      try {
        await new Promise(resolveDelay => {
          setTimeout(resolveDelay, 100)
        })
        expect(acquired).toBe(false)
      } finally {
        await rm(lockPath, { force: true })
        await operation
      }
    } finally {
      await rm(cacheDir, { force: true, recursive: true })
    }
  },
)
