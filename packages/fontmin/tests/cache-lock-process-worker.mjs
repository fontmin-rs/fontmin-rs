import { appendFile, mkdir, writeFile } from 'node:fs/promises'
import { resolve } from 'node:path'
import { withCacheLock } from '../bin/cache-lock.mjs'

const [mode, cacheRoot, eventPath] = process.argv.slice(2)

if (mode === undefined || cacheRoot === undefined) {
  throw new Error('cache lock process worker requires a mode and cache root')
}

await withCacheLock(cacheRoot, async () => {
  if (mode === 'hold') {
    const entryDir = resolve(cacheRoot, 'aa', 'bb', 'interrupted-entry')

    await mkdir(entryDir, { recursive: true })
    await writeFile(
      resolve(entryDir, `index.json.${process.pid}.0.tmp`),
      'incomplete',
    )
    process.stdout.write('acquired\n')
    await new Promise(resolveHold => {
      setTimeout(resolveHold, 60_000)
    })
  }

  if (mode === 'record') {
    if (eventPath === undefined) {
      throw new Error('record mode requires an event path')
    }

    await appendFile(eventPath, `start:${process.pid}\n`)
    await new Promise(resolveDelay => {
      setTimeout(resolveDelay, 50)
    })
    await appendFile(eventPath, `end:${process.pid}\n`)
  }
})
