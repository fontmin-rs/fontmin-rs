import { createHash } from 'node:crypto'
import { readFile, readdir, rm, writeFile, mkdir } from 'node:fs/promises'
import { basename, dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { fuzzOperations } from './fuzz-operations.mjs'

const workspaceRoot = dirname(import.meta.dirname)
const malformedManifestPath = 'fixtures/malformed/manifest.json'
const regressionManifestPath = 'fuzz/regressions/public_api/manifest.json'
const operationCount = fuzzOperations.length
const validSeeds = [
  {
    operations: [0, 1, 2, 3, 4, 6, 9, 10, 11, 12],
    path: 'fixtures/fonts/ttf/noto-sans-sc-compact.ttf',
  },
  {
    operations: [0, 7],
    path: 'fixtures/fonts/otf/font-awesome-free-solid-900.otf',
  },
]

async function clearGeneratedSeeds(outputDirectory) {
  let entries

  try {
    entries = await readdir(outputDirectory)
  } catch (error) {
    if (error?.code === 'ENOENT') {
      return
    }
    throw error
  }

  await Promise.all(
    entries
      .filter(name => name.startsWith('seed-'))
      .map(name => rm(join(outputDirectory, name))),
  )
}

async function writeSeed(outputDirectory, name, operation, contents) {
  const output = Buffer.concat([Buffer.from([operation]), contents])

  await writeFile(join(outputDirectory, `seed-${operation}-${name}`), output)
}

async function readMalformedFixture(root, testCase) {
  const contents = await readFile(join(root, testCase.path))

  if (testCase.encoding === undefined) {
    return contents
  }
  if (testCase.encoding === 'hex') {
    const hex = contents.toString('utf8').trim()
    if (!/^(?:[0-9a-f]{2})+$/u.test(hex)) {
      throw new Error(
        `${testCase.path} must contain complete lowercase hex bytes`,
      )
    }
    return Buffer.from(hex, 'hex')
  }

  throw new Error(
    `${testCase.path} uses unsupported encoding ${testCase.encoding}`,
  )
}

export async function prepareFuzzCorpus({
  outputDirectory = join(workspaceRoot, 'fuzz/corpus/public_api'),
  root = workspaceRoot,
} = {}) {
  const manifest = JSON.parse(
    await readFile(join(root, malformedManifestPath), 'utf8'),
  )
  const regressionManifest = JSON.parse(
    await readFile(join(root, regressionManifestPath), 'utf8'),
  )

  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.cases)) {
    throw new Error(`${malformedManifestPath} must use schema version 1`)
  }
  if (
    regressionManifest.schemaVersion !== 1 ||
    !Array.isArray(regressionManifest.cases)
  ) {
    throw new Error(`${regressionManifestPath} must use schema version 1`)
  }

  await mkdir(outputDirectory, { recursive: true })
  await clearGeneratedSeeds(outputDirectory)

  let count = 0
  for (const testCase of manifest.cases) {
    const contents = await readMalformedFixture(root, testCase)
    const name = basename(testCase.path)

    for (let operation = 0; operation < operationCount; operation += 1) {
      await writeSeed(outputDirectory, `malformed-${name}`, operation, contents)
      count += 1
    }
  }

  for (const seed of validSeeds) {
    const contents = await readFile(join(root, seed.path))
    const name = `valid-${basename(seed.path)}`

    for (const operation of seed.operations) {
      await writeSeed(outputDirectory, name, operation, contents)
      count += 1
    }
  }

  for (const testCase of regressionManifest.cases) {
    const contents = await readFile(join(root, testCase.path))
    const digest = createHash('sha256').update(contents).digest('hex')

    if (digest !== testCase.sha256) {
      throw new Error(
        `${testCase.path} digest is ${digest}; expected ${testCase.sha256}`,
      )
    }
    if (contents.length === 0) {
      throw new Error(`${testCase.path} must include an operation byte`)
    }
    if (contents[0] % operationCount !== testCase.operation.id) {
      throw new Error(`${testCase.path} operation metadata does not match`)
    }

    await writeFile(
      join(outputDirectory, `seed-regression-${testCase.sha256}.bin`),
      contents,
    )
    count += 1
  }

  return { count, outputDirectory: resolve(outputDirectory) }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const result = await prepareFuzzCorpus()

  console.log(
    `Prepared ${result.count} fuzz seeds in ${result.outputDirectory}.`,
  )
}
