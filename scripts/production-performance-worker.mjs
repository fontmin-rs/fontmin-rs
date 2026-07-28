import { readFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { performance } from 'node:perf_hooks'

const workspaceRoot = dirname(import.meta.dirname)
const stageName = process.argv[2]

if (stageName === undefined) {
  throw new Error('production performance worker requires a stage name')
}

const [budgets, fixtureManifest] = await Promise.all([
  readFile(
    join(workspaceRoot, 'benchmarks/production-budgets.json'),
    'utf8',
  ).then(JSON.parse),
  readFile(
    join(workspaceRoot, 'fixtures/production/manifest.json'),
    'utf8',
  ).then(JSON.parse),
])
const stage = budgets.stages.find(candidate => candidate.name === stageName)

if (stage === undefined) {
  throw new Error(`unknown production performance stage: ${stageName}`)
}

const fixture =
  stage.fixtureId === undefined
    ? undefined
    : fixtureManifest.fixtures.find(
        candidate => candidate.id === stage.fixtureId,
      )
if (stage.fixtureId !== undefined && fixture === undefined) {
  throw new Error(`unknown production fixture: ${stage.fixtureId}`)
}

const contents =
  fixture === undefined
    ? undefined
    : await readFile(
        join(workspaceRoot, 'fixtures/production/.cache', fixture.cachePath),
      )
const modulePath =
  stage.runtime === 'native'
    ? 'packages/fontmin/dist/index.mjs'
    : 'wasm/fontmin/dist/index.mjs'
const runtime = await import(new URL(`../${modulePath}`, import.meta.url).href)
const wasmBytes =
  stage.runtime === 'wasm'
    ? await readFile(
        join(workspaceRoot, 'wasm/fontmin/dist/fontmin_wasm_core_bg.wasm'),
      )
    : undefined

if (stage.runtime === 'wasm' && stage.operation !== 'init') {
  await runtime.initWasm(wasmBytes)
}

const rssBeforeMiB = process.memoryUsage().rss / 1024 / 1024
const startedAt = performance.now()
let outputBytes = 0

if (stage.operation === 'init') {
  await runtime.initWasm(wasmBytes)
  outputBytes = wasmBytes.byteLength
} else if (stage.operation === 'inspect') {
  const info = await runtime.inspect(contents)

  outputBytes = Buffer.byteLength(JSON.stringify(info))
} else if (stage.operation === 'mixed-delivery') {
  const slices = [
    { name: 'latin', unicodeRanges: ['U+0020-007E'] },
    { name: 'cjk', unicodeRanges: ['U+4E00-4E7F'] },
    { name: 'punctuation', unicodeRanges: ['U+3000-303F'] },
  ]
  const assets =
    stage.runtime === 'native'
      ? await runtime.optimize({
          cache: false,
          input: [
            resolve(
              workspaceRoot,
              'fixtures/production/.cache',
              fixture.cachePath,
            ),
          ],
          plugins: [runtime.deliverySlices(slices)],
        })
      : await runtime.optimizeBrowser({
          assets: [{ contents, fileName: fixture.cachePath }],
          plugins: [runtime.deliverySlices(slices)],
        })

  outputBytes = assets.reduce(
    (total, asset) => total + asset.contents.byteLength,
    0,
  )
} else {
  throw new Error(`unsupported performance operation: ${stage.operation}`)
}

const latencyMs = performance.now() - startedAt
const rssAfterMiB = process.memoryUsage().rss / 1024 / 1024
const maxRssMiB = process.resourceUsage().maxRSS / 1024

console.log(
  JSON.stringify({
    latencyMs,
    maxRssMiB,
    outputBytes,
    rssAfterMiB,
    rssBeforeMiB,
  }),
)
