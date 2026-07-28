import { createHash, randomUUID } from 'node:crypto'
import { createWriteStream } from 'node:fs'
import { mkdir, readFile, readdir, rename, rm } from 'node:fs/promises'
import { basename, dirname, join, resolve } from 'node:path'
import { Readable, Transform } from 'node:stream'
import { pipeline } from 'node:stream/promises'
import { pathToFileURL } from 'node:url'

const workspaceRoot = dirname(import.meta.dirname)
const manifestRelativePath = 'fixtures/production/manifest.json'
const cacheRelativePath = 'fixtures/production/.cache'
const digestPattern = /^[\da-f]{64}$/u

function fixtureDigest(contents) {
  return createHash('sha256').update(contents).digest('hex')
}

function assertFixture(fixture) {
  if (
    typeof fixture.id !== 'string' ||
    fixture.id.length === 0 ||
    typeof fixture.cachePath !== 'string' ||
    basename(fixture.cachePath) !== fixture.cachePath ||
    typeof fixture.downloadUrl !== 'string' ||
    !digestPattern.test(fixture.sha256 ?? '') ||
    !Number.isSafeInteger(fixture.byteLength) ||
    fixture.byteLength <= 0
  ) {
    throw new Error(`${manifestRelativePath} contains an invalid fixture`)
  }
}

function assertByteMetadata(fixture, byteLength, digest) {
  if (byteLength !== fixture.byteLength) {
    throw new Error(
      `${fixture.id} byte length is ${byteLength}; expected ${fixture.byteLength}`,
    )
  }

  if (digest !== fixture.sha256) {
    throw new Error(
      `${fixture.id} digest is ${digest}; expected ${fixture.sha256}`,
    )
  }
}

function assertContents(fixture, contents) {
  assertByteMetadata(fixture, contents.length, fixtureDigest(contents))
}

async function hasValidCacheEntry(path, fixture) {
  let contents

  try {
    contents = await readFile(path)
  } catch (error) {
    if (error?.code === 'ENOENT') {
      return false
    }
    throw error
  }

  try {
    assertContents(fixture, contents)
    return true
  } catch {
    return false
  }
}

function isRunningProcess(pid) {
  try {
    process.kill(pid, 0)
    return true
  } catch (error) {
    return error?.code !== 'ESRCH'
  }
}

async function clearStaleTemporaryFiles(cacheDirectory, fixture) {
  const prefix = `.${fixture.cachePath}.`
  const entries = await readdir(cacheDirectory)

  await Promise.all(
    entries
      .filter(name => name.startsWith(prefix) && name.endsWith('.tmp'))
      .filter(name => {
        const [pidText] = name.slice(prefix.length).split('.', 1)
        const pid = Number(pidText)

        return !Number.isSafeInteger(pid) || !isRunningProcess(pid)
      })
      .map(name => rm(join(cacheDirectory, name), { force: true })),
  )
}

async function downloadFixture({ cacheDirectory, fetchImpl, fixture, path }) {
  const temporaryPath = join(
    cacheDirectory,
    `.${fixture.cachePath}.${process.pid}.${randomUUID()}.tmp`,
  )

  try {
    const response = await fetchImpl(fixture.downloadUrl, {
      headers: {
        accept: 'application/octet-stream',
        'accept-encoding': 'identity',
        'user-agent': 'fontmin-rs-production-corpus',
      },
    })

    if (!response.ok) {
      throw new Error(
        `failed to download ${fixture.id}: HTTP ${response.status}`,
      )
    }
    if (response.body === null) {
      throw new Error(`failed to download ${fixture.id}: empty response body`)
    }

    const hash = createHash('sha256')
    let byteLength = 0
    const verifier = new Transform({
      transform(chunk, _encoding, callback) {
        byteLength += chunk.length
        hash.update(chunk)
        callback(null, chunk)
      },
    })

    await pipeline(
      Readable.fromWeb(response.body),
      verifier,
      createWriteStream(temporaryPath, { flags: 'wx' }),
    )
    assertByteMetadata(fixture, byteLength, hash.digest('hex'))
    await rm(path, { force: true })
    await rename(temporaryPath, path)
  } finally {
    await rm(temporaryPath, { force: true })
  }
}

export async function prepareProductionFixtures({
  fetchImpl = globalThis.fetch,
  root = workspaceRoot,
} = {}) {
  const manifest = JSON.parse(
    await readFile(join(root, manifestRelativePath), 'utf8'),
  )

  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.fixtures)) {
    throw new Error(`${manifestRelativePath} must use schema version 1`)
  }

  const cacheDirectory = join(root, cacheRelativePath)

  await mkdir(cacheDirectory, { recursive: true })

  const fixtures = []
  for (const fixture of manifest.fixtures) {
    assertFixture(fixture)
    const path = join(cacheDirectory, fixture.cachePath)

    await clearStaleTemporaryFiles(cacheDirectory, fixture)

    if (await hasValidCacheEntry(path, fixture)) {
      fixtures.push({ id: fixture.id, path: resolve(path), status: 'reused' })
      continue
    }

    await downloadFixture({ cacheDirectory, fetchImpl, fixture, path })
    fixtures.push({ id: fixture.id, path: resolve(path), status: 'downloaded' })
  }

  return { cacheDirectory: resolve(cacheDirectory), fixtures }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const result = await prepareProductionFixtures()
  const downloaded = result.fixtures.filter(
    fixture => fixture.status === 'downloaded',
  ).length

  console.log(
    `Prepared ${result.fixtures.length} production fixtures (${downloaded} downloaded, ${result.fixtures.length - downloaded} reused).`,
  )
}
