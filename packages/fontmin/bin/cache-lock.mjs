import { randomUUID } from 'node:crypto'
import { mkdir, open, readFile, readdir, rm, stat } from 'node:fs/promises'
import { join } from 'node:path'

const CACHE_LOCK_RETRY_COUNT = 200
const CACHE_LOCK_RETRY_MS = 25
const CACHE_LOCK_STALE_MS = 5 * 60_000
export async function withCacheLock(cacheRoot, operation) {
  const lockPath = join(cacheRoot, '.write.lock')
  const owner = `${process.pid}:${randomUUID()}`
  let recoveredLock = false

  await mkdir(cacheRoot, { recursive: true })

  for (let attempt = 0; attempt < CACHE_LOCK_RETRY_COUNT; attempt += 1) {
    let lock

    try {
      lock = await open(lockPath, 'wx')
    } catch (error) {
      if (!hasErrorCode(error, 'EEXIST')) {
        throw error
      }

      const staleOwner = await readStaleLockOwner(lockPath)

      if (staleOwner !== undefined) {
        recoveredLock =
          (await removeLockIfOwned(lockPath, staleOwner)) || recoveredLock
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

    if (recoveredLock) {
      try {
        await cleanupCacheTemporaryFiles(cacheRoot)
      } catch (error) {
        await lock.close()
        await removeLockIfOwned(lockPath, owner)
        throw error
      }
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

async function readStaleLockOwner(path) {
  try {
    const [owner, lockStat] = await Promise.all([
      readFile(path, 'utf8'),
      stat(path),
    ])

    const ownerPid = parseOwnerPid(owner)

    return (ownerPid !== undefined && !isProcessAlive(ownerPid)) ||
      Date.now() - lockStat.mtimeMs > CACHE_LOCK_STALE_MS
      ? owner
      : undefined
  } catch (error) {
    if (hasErrorCode(error, 'ENOENT')) {
      return
    }

    throw error
  }
}

async function cleanupCacheTemporaryFiles(cacheRoot) {
  await cleanupTemporaryFilesInTree(cacheRoot)
}

async function cleanupTemporaryFilesInTree(path) {
  let entries

  try {
    entries = await readdir(path, { withFileTypes: true })
  } catch (error) {
    if (hasErrorCode(error, 'ENOENT')) {
      return
    }

    throw error
  }

  await Promise.all(
    entries.map(entry => {
      const entryPath = join(path, entry.name)

      if (entry.isDirectory()) {
        return cleanupTemporaryFilesInTree(entryPath)
      }

      return entry.isFile() && entry.name.endsWith('.tmp')
        ? rm(entryPath, { force: true })
        : Promise.resolve()
    }),
  )
}

function parseOwnerPid(owner) {
  const pid = Math.trunc(Number(owner.split(':', 1)[0] ?? ''))

  return Number.isSafeInteger(pid) && pid > 0 ? pid : undefined
}

function isProcessAlive(pid) {
  try {
    process.kill(pid, 0)
    return true
  } catch (error) {
    return !hasErrorCode(error, 'ESRCH')
  }
}

async function removeLockIfOwned(path, owner) {
  try {
    if ((await readFile(path, 'utf8')) !== owner) {
      return false
    }

    await rm(path)
    return true
  } catch (error) {
    if (hasErrorCode(error, 'ENOENT')) {
      return false
    }

    throw error
  }
}

function hasErrorCode(error, code) {
  return (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    error.code === code
  )
}

function delay(milliseconds) {
  return new Promise(resolveDelay => {
    setTimeout(resolveDelay, milliseconds)
  })
}
