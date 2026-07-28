import { spawn } from 'node:child_process'
import type { ChildProcess } from 'node:child_process'
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { expect, it } from 'vitest'

const workerPath = fileURLToPath(
  new URL('cache-lock-process-worker.mjs', import.meta.url),
)

function runWorker(
  mode: 'hold' | 'record',
  cacheRoot: string,
  eventPath?: string,
): ChildProcess {
  return spawn(
    process.execPath,
    [
      workerPath,
      mode,
      cacheRoot,
      ...(eventPath === undefined ? [] : [eventPath]),
    ],
    {
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  )
}

function waitForExit(child: ChildProcess): Promise<void> {
  return new Promise((resolveExit, reject) => {
    child.once('error', reject)
    child.once('exit', (code, signal) => {
      if (code === 0) {
        resolveExit()
      } else {
        reject(new Error(`worker exited with ${signal ?? `code ${code}`}`))
      }
    })
  })
}

function waitForAcquisition(child: ChildProcess): Promise<void> {
  return new Promise((resolveAcquisition, reject) => {
    child.once('error', reject)
    child.stdout?.on('data', chunk => {
      if (String(chunk).includes('acquired')) {
        resolveAcquisition()
      }
    })
    child.once('exit', code => {
      reject(new Error(`holder exited before acquisition with code ${code}`))
    })
  })
}

it('serializes independent cache writers', async () => {
  const root = await mkdtemp(resolve(tmpdir(), 'fontmin-cache-processes-'))
  const cacheRoot = resolve(root, 'v1')
  const eventPath = resolve(root, 'events.log')

  try {
    await Promise.all(
      Array.from({ length: 4 }, () =>
        waitForExit(runWorker('record', cacheRoot, eventPath)),
      ),
    )

    const eventContents = await readFile(eventPath, 'utf8')
    const events = eventContents.trim().split('\n')
    let active = 0

    for (const event of events) {
      active += event.startsWith('start:') ? 1 : -1
      expect(active).toBeGreaterThanOrEqual(0)
      expect(active).toBeLessThanOrEqual(1)
    }

    expect(active).toBe(0)
    await expect(readdir(cacheRoot)).resolves.not.toContain('.write.lock')
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

it('reclaims a terminated writer and removes its temporary files', async () => {
  const root = await mkdtemp(resolve(tmpdir(), 'fontmin-cache-recovery-'))
  const cacheRoot = resolve(root, 'v1')
  const eventPath = resolve(root, 'events.log')
  const holder = runWorker('hold', cacheRoot)

  try {
    await waitForAcquisition(holder)
    const holderExit = new Promise(resolveExit => {
      holder.once('exit', resolveExit)
    })
    holder.kill()
    await holderExit

    await waitForExit(runWorker('record', cacheRoot, eventPath))

    const entryFiles = await readdir(
      resolve(cacheRoot, 'aa', 'bb', 'interrupted-entry'),
    )

    expect(entryFiles.some(file => file.endsWith('.tmp'))).toBe(false)
    await expect(readdir(cacheRoot)).resolves.not.toContain('.write.lock')
  } finally {
    holder.kill()
    await rm(root, { force: true, recursive: true })
  }
}, 10_000)
