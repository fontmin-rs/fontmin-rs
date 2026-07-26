import { randomUUID } from 'node:crypto'
import { mkdir, open, readFile, rm, stat } from 'node:fs/promises'
import { join } from 'node:path'

const CACHE_LOCK_RETRY_COUNT = 200
const CACHE_LOCK_RETRY_MS = 25
const CACHE_LOCK_STALE_MS = 5 * 60_000
export async function withCacheLock<T>(
  cacheRoot: string,
  operation: () => Promise<T>,
): Promise<T> {
  const lockPath = join(cacheRoot, '.write.lock')
  const owner = `${process.pid}:${randomUUID()}`

  await mkdir(cacheRoot, { recursive: true })

  for (let attempt = 0; attempt < CACHE_LOCK_RETRY_COUNT; attempt += 1) {
    let lock

    try {
      lock = await open(lockPath, 'wx')
    } catch (error) {
      if (!isNodeErrorWithCode(error, 'EEXIST')) {
        throw error
      }

      const staleOwner = await readStaleLockOwner(lockPath)

      if (staleOwner !== undefined) {
        await removeLockIfOwned(lockPath, staleOwner)
        continue
      }

      await delay(CACHE_LOCK_RETRY_MS)
      continue
    }

    try {
      await lock.writeFile(owner)
    } catch (error) {
      await lock.close()
      await rm(lockPath, { force: true })
      throw error
    }

    try {
      return await operation()
    } finally {
      await lock.close()
      await removeLockIfOwned(lockPath, owner)
    }
  }

  throw new Error(`timed out waiting for cache write lock: ${lockPath}`)
}

async function readStaleLockOwner(path: string): Promise<string | undefined> {
  try {
    const [owner, lockStat] = await Promise.all([
      readFile(path, 'utf8'),
      stat(path),
    ])

    return Date.now() - lockStat.mtimeMs > CACHE_LOCK_STALE_MS
      ? owner
      : undefined
  } catch (error) {
    if (isNodeErrorWithCode(error, 'ENOENT')) {
      return
    }

    throw error
  }
}

async function removeLockIfOwned(
  path: string,
  owner: string,
): Promise<boolean> {
  try {
    if ((await readFile(path, 'utf8')) !== owner) {
      return false
    }

    await rm(path)
    return true
  } catch (error) {
    if (isNodeErrorWithCode(error, 'ENOENT')) {
      return false
    }

    throw error
  }
}

function isNodeErrorWithCode(error: unknown, code: string): boolean {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    error.code === code
  )
}

function delay(milliseconds: number): Promise<void> {
  return new Promise(resolveDelay => {
    setTimeout(resolveDelay, milliseconds)
  })
}
