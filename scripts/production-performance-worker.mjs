import { execFile, spawn } from 'node:child_process'
import {
  access,
  link,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { platform, tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { performance } from 'node:perf_hooks'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)

const workspaceRoot = dirname(import.meta.dirname)
const stageName = process.argv[2]

async function sampleProcessRssMiB(pid) {
  try {
    if (platform() === 'linux') {
      const status = await readFile(`/proc/${pid}/status`, 'utf8')
      const peak = /^VmHWM:\s+(?<rssKiB>\d+)\s+kB$/mu.exec(status)
      const current = /^VmRSS:\s+(?<rssKiB>\d+)\s+kB$/mu.exec(status)
      const rssKiB = Number(peak?.groups?.rssKiB ?? current?.groups?.rssKiB)

      return Number.isFinite(rssKiB) ? rssKiB / 1024 : 0
    }

    const { stdout } = await executeFile('ps', [
      '-o',
      'rss=',
      '-p',
      String(pid),
    ])
    const rssKiB = Number(stdout.trim())

    return Number.isFinite(rssKiB) ? rssKiB / 1024 : 0
  } catch {
    return 0
  }
}

async function runMonitoredProcess(file, arguments_, options) {
  const child = spawn(file, arguments_, {
    ...options,
    stdio: ['ignore', 'pipe', 'pipe'],
  })
  let maxRssMiB = 0
  let stderr = ''
  let stdout = ''
  let activeSample

  child.stderr.setEncoding('utf8')
  child.stdout.setEncoding('utf8')
  child.stderr.on('data', chunk => {
    stderr = `${stderr}${chunk}`.slice(-64 * 1024)
  })
  child.stdout.on('data', chunk => {
    stdout = `${stdout}${chunk}`.slice(-64 * 1024)
  })

  const sample = async () => {
    if (activeSample !== undefined) {
      await activeSample
      return
    }
    if (child.pid === undefined) {
      return
    }

    const sampleRequest = sampleProcessRssMiB(child.pid)
    activeSample = sampleRequest
    maxRssMiB = Math.max(maxRssMiB, await sampleRequest)
    activeSample = undefined
  }
  const exit = new Promise((resolveExit, rejectExit) => {
    child.once('error', rejectExit)
    child.once('close', resolveExit)
  })

  await sample()
  const sampler = setInterval(sample, 20)

  let exitCode
  try {
    exitCode = await exit
  } finally {
    clearInterval(sampler)
    await sample()
  }

  if (exitCode !== 0) {
    throw new Error(
      `process exited with status ${exitCode}: ${stderr.trim() || stdout.trim()}`,
    )
  }

  return { maxRssMiB }
}

async function createScaleInputs(root, inputCount) {
  const inputs = join(root, 'inputs')
  const seed = join(root, 'seed.bin')

  await mkdir(inputs)
  await writeFile(seed, Uint8Array.of(0))
  for (let start = 0; start < inputCount; start += 256) {
    const end = Math.min(start + 256, inputCount)

    await Promise.all(
      Array.from({ length: end - start }, (_, offset) => {
        const index = start + offset
        const fileName = `input-${String(index).padStart(5, '0')}.bin`

        return link(seed, join(inputs, fileName))
      }),
    )
  }
}

async function measureRustBuildScale(stage) {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-build-scale-'))
  const inputCount = stage.inputCount ?? 10_000
  const threads = stage.threads ?? 4
  const outputDir = join(root, 'output')
  const configPath = join(root, 'fontmin.config.json')
  const executable = join(
    workspaceRoot,
    'target',
    'release',
    platform() === 'win32' ? 'fontmin-rs.exe' : 'fontmin-rs',
  )

  try {
    await access(executable)
    await createScaleInputs(root, inputCount)
    await writeFile(
      configPath,
      `${JSON.stringify({
        cache: { dir: 'cache', enabled: false },
        css: null,
        cwd: root,
        diagnostics: { level: 'silent' },
        input: ['inputs/*.bin'],
        outDir: 'output',
        outputs: [],
        parallel: { perFile: true, threads: { count: threads } },
        plugins: [],
      })}\n`,
    )

    const startedAt = performance.now()
    const { maxRssMiB } = await runMonitoredProcess(
      executable,
      ['build', '--config', configPath],
      { cwd: workspaceRoot },
    )
    const latencyMs = performance.now() - startedAt
    const outputs = await readdir(outputDir)

    if (outputs.length !== inputCount) {
      throw new Error(
        `build-scale produced ${outputs.length} outputs for ${inputCount} inputs`,
      )
    }

    return { latencyMs, maxRssMiB, outputBytes: outputs.length }
  } finally {
    await rm(root, { force: true, recursive: true })
  }
}

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
const runtime =
  stage.runtime === 'rust-cli'
    ? undefined
    : await import(new URL(`../${modulePath}`, import.meta.url).href)
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
let eventLoopLagMs
let measuredLatencyMs
let measuredMaxRssMiB

if (stage.operation === 'build-scale') {
  const measurement = await measureRustBuildScale(stage)

  measuredLatencyMs = measurement.latencyMs
  measuredMaxRssMiB = measurement.maxRssMiB
  outputBytes = measurement.outputBytes
} else if (stage.operation === 'init') {
  await runtime.initWasm(wasmBytes)
  outputBytes = wasmBytes.byteLength
} else if (stage.operation === 'inspect') {
  const info = await runtime.inspect(contents)

  outputBytes = Buffer.byteLength(JSON.stringify(info))
} else if (stage.operation === 'async-woff2') {
  const timerStartedAt = performance.now()
  const timer = new Promise(resolveTimer => {
    setTimeout(() => {
      resolveTimer(performance.now() - timerStartedAt)
    }, 0)
  })
  const output = await runtime.ttfToWoff2Async(contents)

  eventLoopLagMs = await timer
  outputBytes = output.byteLength
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
} else if (stage.operation === 'multi-file') {
  const inputCopies = stage.inputCopies ?? 4
  const assets = await runtime.optimize({
    input: Array.from({ length: inputCopies }, () => contents),
    outputs: ['ttf'],
    subset: { text: 'Hello, 世界' },
  })

  outputBytes = assets.reduce(
    (total, asset) => total + asset.contents.byteLength,
    0,
  )
} else if (stage.operation === 'auto-delivery') {
  const assets = await runtime.optimize({
    input: [
      resolve(workspaceRoot, 'fixtures/production/.cache', fixture.cachePath),
    ],
    plugins: [
      runtime.autoDeliverySlices({
        languages: ['zh-Hans'],
        maxSlices: 8,
        measureFormat: 'ttf',
        targetBytes: 512 * 1024,
      }),
    ],
  })

  outputBytes = assets.reduce(
    (total, asset) => total + asset.contents.byteLength,
    0,
  )
} else if (stage.operation === 'cache-scale') {
  const cacheDir = await mkdtemp(join(tmpdir(), 'fontmin-cache-scale-'))

  try {
    const entryCount = stage.entryCount ?? 512
    const concurrency = stage.concurrency ?? 1

    for (let start = 0; start < entryCount; start += concurrency) {
      const end = Math.min(start + concurrency, entryCount)

      await Promise.all(
        Array.from({ length: end - start }, (_, offset) => {
          const index = start + offset

          return runtime.optimize({
            cache: { dir: cacheDir },
            input: [
              Uint8Array.of(
                index % 256,
                Math.floor(index / 256) % 256,
                Math.floor(index / 65_536) % 256,
              ),
            ],
            outputs: [],
          })
        }),
      )
    }

    const cacheIndex = await readFile(join(cacheDir, 'v1', 'index.json'))
    outputBytes = cacheIndex.byteLength
  } finally {
    await rm(cacheDir, { force: true, recursive: true })
  }
} else {
  throw new Error(`unsupported performance operation: ${stage.operation}`)
}

const latencyMs = measuredLatencyMs ?? performance.now() - startedAt
const rssAfterMiB = process.memoryUsage().rss / 1024 / 1024
const maxRssMiB = measuredMaxRssMiB ?? process.resourceUsage().maxRSS / 1024

console.log(
  JSON.stringify({
    latencyMs,
    ...(eventLoopLagMs === undefined ? {} : { eventLoopLagMs }),
    maxRssMiB,
    outputBytes,
    rssAfterMiB,
    rssBeforeMiB,
  }),
)
