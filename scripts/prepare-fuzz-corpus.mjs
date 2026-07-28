import { createHash } from 'node:crypto'
import { readFile, readdir, rm, writeFile, mkdir } from 'node:fs/promises'
import { basename, dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import {
  focusedFuzzTargets,
  fuzzOperations,
  fuzzTargetOperations,
} from './fuzz-operations.mjs'

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
const focusedValidPaths = [
  'fixtures/fonts/ttf/noto-sans-sc-compact.ttf',
  'fixtures/fonts/ttf/roboto-regular.ttf',
  'fixtures/fonts/otf/font-awesome-free-solid-900.otf',
]
const configurationSeeds = [
  ['empty', '{}'],
  [
    'valid-modern-web',
    JSON.stringify({
      input: ['fonts/*.ttf'],
      outputs: [{ format: 'woff2' }],
      subset: { basicText: true, text: 'A中' },
    }),
  ],
  [
    'valid-plugin',
    JSON.stringify({
      plugins: [{ name: 'glyph', native: { name: 'glyph', text: 'A中' } }],
    }),
  ],
  ['nested-invalid', '{"outputs":[{"format":{"unexpected":[[[[]]]]}}]}'],
]
const outputNamingSeeds = [
  { name: 'safe-nested', operation: 0, value: 'nested/font.woff2' },
  { name: 'traversal-parent', operation: 0, value: '../font.ttf' },
  { name: 'traversal-nested', operation: 0, value: 'fonts/../../font.ttf' },
  { name: 'absolute', operation: 0, value: '/tmp/font.ttf' },
  { name: 'empty', operation: 0, value: '' },
  { name: 'valid-extension', operation: 1, value: '.woff2' },
  { name: 'traversal-extension', operation: 1, value: '../woff2' },
  { name: 'separator-extension', operation: 1, value: 'font/woff2' },
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
  let malformedSeedCount = 0
  for (const testCase of manifest.cases) {
    const contents = await readMalformedFixture(root, testCase)
    const name = basename(testCase.path)

    for (let operation = 0; operation < operationCount; operation += 1) {
      await writeSeed(outputDirectory, `malformed-${name}`, operation, contents)
      count += 1
      malformedSeedCount += 1
    }
  }

  let validSeedCount = 0
  for (const seed of validSeeds) {
    const contents = await readFile(join(root, seed.path))
    const name = `valid-${basename(seed.path)}`

    for (const operation of seed.operations) {
      await writeSeed(outputDirectory, name, operation, contents)
      count += 1
      validSeedCount += 1
    }
  }

  let regressionSeedCount = 0
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
    regressionSeedCount += 1
  }

  return {
    count,
    malformedSeedCount,
    outputDirectory: resolve(outputDirectory),
    regressionSeedCount,
    validSeedCount,
  }
}

async function addPermanentRegressions({ outputDirectory, root, target }) {
  const manifestPath = `fuzz/regressions/${target}/manifest.json`
  const manifest = JSON.parse(await readFile(join(root, manifestPath), 'utf8'))

  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.cases)) {
    throw new Error(`${manifestPath} must use schema version 1`)
  }

  for (const testCase of manifest.cases) {
    const contents = await readFile(join(root, testCase.path))
    const digest = createHash('sha256').update(contents).digest('hex')

    if (digest !== testCase.sha256) {
      throw new Error(
        `${testCase.path} digest is ${digest}; expected ${testCase.sha256}`,
      )
    }

    await writeFile(
      join(outputDirectory, `seed-regression-${testCase.sha256}.bin`),
      contents,
    )
  }

  return manifest.cases.length
}

async function prepareBinaryTarget({
  malformedManifest,
  outputDirectory,
  root,
  target,
}) {
  const operations = fuzzTargetOperations(target)

  await mkdir(outputDirectory, { recursive: true })
  await clearGeneratedSeeds(outputDirectory)

  let count = 0
  for (const testCase of malformedManifest.cases) {
    const contents = await readMalformedFixture(root, testCase)
    const name = `malformed-${basename(testCase.path)}`

    for (let operation = 0; operation < operations.length; operation += 1) {
      await writeSeed(outputDirectory, name, operation, contents)
      count += 1
    }
  }

  for (const path of focusedValidPaths) {
    const contents = await readFile(join(root, path))
    const name = `valid-${basename(path)}`

    for (let operation = 0; operation < operations.length; operation += 1) {
      await writeSeed(outputDirectory, name, operation, contents)
      count += 1
    }
  }

  count += await addPermanentRegressions({
    outputDirectory,
    root,
    target,
  })

  return count
}

async function prepareTextTarget({ outputDirectory, root, seeds, target }) {
  await mkdir(outputDirectory, { recursive: true })
  await clearGeneratedSeeds(outputDirectory)

  for (const seed of seeds) {
    await writeSeed(
      outputDirectory,
      seed.name,
      seed.operation ?? 0,
      Buffer.from(seed.value),
    )
  }

  const regressionCount = await addPermanentRegressions({
    outputDirectory,
    root,
    target,
  })

  return seeds.length + regressionCount
}

export async function prepareFocusedFuzzCorpora({
  corpusRoot = join(workspaceRoot, 'fuzz/corpus'),
  root = workspaceRoot,
} = {}) {
  const malformedManifest = JSON.parse(
    await readFile(join(root, malformedManifestPath), 'utf8'),
  )

  if (
    malformedManifest.schemaVersion !== 1 ||
    !Array.isArray(malformedManifest.cases)
  ) {
    throw new Error(`${malformedManifestPath} must use schema version 1`)
  }

  const targets = []
  for (const target of focusedFuzzTargets) {
    const outputDirectory = join(corpusRoot, target)
    let count

    if (target === 'configuration') {
      count = await prepareTextTarget({
        outputDirectory,
        root,
        seeds: configurationSeeds.map(([name, value]) => ({
          name: `valid-${name}`,
          value,
        })),
        target,
      })
    } else if (target === 'output_naming') {
      count = await prepareTextTarget({
        outputDirectory,
        root,
        seeds: outputNamingSeeds,
        target,
      })
    } else {
      count = await prepareBinaryTarget({
        malformedManifest,
        outputDirectory,
        root,
        target,
      })
    }

    targets.push({
      count,
      name: target,
      outputDirectory: resolve(outputDirectory),
    })
  }

  return {
    count: targets.reduce((total, target) => total + target.count, 0),
    targets,
  }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const [legacy, focused] = await Promise.all([
    prepareFuzzCorpus(),
    prepareFocusedFuzzCorpora(),
  ])

  console.log(
    `Prepared ${legacy.count + focused.count} fuzz seeds across ${focused.targets.length + 1} targets.`,
  )
}
