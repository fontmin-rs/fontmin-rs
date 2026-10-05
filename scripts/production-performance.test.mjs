import assert from 'node:assert/strict'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { runProductionPerformance } from './production-performance.mjs'

const stages = [
  {
    fixtureId: 'test-font',
    maxEventLoopLagMs: 20,
    maxLatencyMs: 100,
    maxRssMiB: 64,
    name: 'native:inspect:test-font',
    operation: 'inspect',
    runtime: 'native',
  },
  {
    fixtureId: 'test-font',
    maxLatencyMs: 200,
    maxRssMiB: 128,
    name: 'wasm:inspect:test-font',
    operation: 'inspect',
    runtime: 'wasm',
  },
  {
    inputCount: 10_000,
    maxLatencyMs: 30_000,
    maxRssMiB: 256,
    name: 'rust-cli:build-scale:10000',
    operation: 'build-scale',
    runtime: 'rust-cli',
    threads: 4,
  },
]

async function createWorkspace(configuredStages = stages) {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-production-performance-'))

  await mkdir(join(root, 'benchmarks'), { recursive: true })
  await writeFile(
    join(root, 'package.json'),
    JSON.stringify({ version: '0.3.0' }),
  )
  await writeFile(
    join(root, 'benchmarks/production-budgets.json'),
    JSON.stringify({
      profile: 'test',
      schemaVersion: 1,
      stages: configuredStages,
      trials: 3,
    }),
  )

  return root
}

test('writes a stage-attributed production performance report', async () => {
  const root = await createWorkspace()
  const output = join(root, 'benchmarks/current.json')

  try {
    const report = await runProductionPerformance({
      executeStage: async stage => ({
        eventLoopLagMs: stage.runtime === 'native' ? 10 : 0,
        latencyMs: stage.runtime === 'native' ? 40 : 80,
        maxRssMiB: stage.runtime === 'native' ? 32 : 96,
        outputBytes: 123,
      }),
      generatedAt: '2026-07-28T00:00:00.000Z',
      output,
      root,
    })

    assert.equal(report.status, 'passed')
    assert.deepEqual(
      report.stages.map(stage => [stage.name, stage.status]),
      [
        ['native:inspect:test-font', 'passed'],
        ['wasm:inspect:test-font', 'passed'],
        ['rust-cli:build-scale:10000', 'passed'],
      ],
    )
    assert.deepEqual(report.stages[0]?.metrics.trialLatencyMs, [40, 40, 40])
    assert.deepEqual(
      report.stages[0]?.metrics.trialEventLoopLagMs,
      [10, 10, 10],
    )
    assert.deepEqual(report.stages[2]?.parameters, {
      inputCount: 10_000,
      threads: 4,
    })
    assert.deepEqual(JSON.parse(await readFile(output, 'utf8')), report)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

const fontBatchStage = {
  cacheState: 'warm',
  fixtureId: 'test-font',
  inputCount: 8,
  maxLatencyMs: 100,
  maxRssMiB: 128,
  name: 'rust-cli:build-font-batch:test-font:4:warm',
  operation: 'build-font-batch',
  runtime: 'rust-cli',
  threads: 4,
}

test('reports font batch parameters, median latency, peak RSS, and stable output digest', async () => {
  const root = await createWorkspace([fontBatchStage])
  let trial = 0

  try {
    const report = await runProductionPerformance({
      executeStage: async () => ({
        latencyMs: [90, 10, 20][trial],
        maxRssMiB: [32, 96, 64][trial++],
        outputBytes: 512,
        outputSha256: 'stable-digest',
      }),
      output: join(root, 'current.json'),
      root,
    })

    assert.deepEqual(report.stages[0].parameters, {
      cacheState: 'warm',
      inputCount: 8,
      threads: 4,
    })
    assert.equal(report.stages[0].metrics.latencyMs, 20)
    assert.equal(report.stages[0].metrics.maxRssMiB, 96)
    assert.equal(report.stages[0].metrics.outputSha256, 'stable-digest')
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects changed output bytes even when the output size stays constant', async () => {
  const root = await createWorkspace([fontBatchStage])
  let trial = 0

  try {
    await assert.rejects(
      runProductionPerformance({
        executeStage: async () => ({
          latencyMs: 10,
          maxRssMiB: 32,
          outputBytes: 512,
          outputSha256: `digest-${trial++}`,
        }),
        output: join(root, 'current.json'),
        root,
      }),
      /output changed between trials/u,
    )
    const report = JSON.parse(
      await readFile(join(root, 'current.json'), 'utf8'),
    )
    assert.equal(report.stages[0].status, 'failed')
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('requires complete and valid real-font batch budget parameters', async () => {
  for (const overrides of [
    { cacheState: undefined },
    { cacheState: 'disabled' },
    { fixtureId: undefined },
    { inputCount: undefined },
    { threads: undefined },
    { runtime: 'native' },
  ]) {
    const root = await createWorkspace([{ ...fontBatchStage, ...overrides }])

    try {
      await assert.rejects(
        runProductionPerformance({ root }),
        /invalid stage|must declare/u,
      )
    } finally {
      await rm(root, { force: true, recursive: true })
    }
  }
})

test('keeps scheduler stress coverage alongside cold and warm real-font batches', async () => {
  const budgets = JSON.parse(
    await readFile(
      new URL('../benchmarks/production-budgets.json', import.meta.url),
      'utf8',
    ),
  )

  assert.ok(
    budgets.stages.some(
      stage => stage.operation === 'build-scale' && stage.inputCount === 10_000,
    ),
  )
  assert.deepEqual(
    budgets.stages
      .filter(stage => stage.operation === 'build-font-batch')
      .map(stage => [stage.inputCount, stage.threads, stage.cacheState]),
    [
      [8, 1, 'cold'],
      [8, 4, 'cold'],
      [8, 1, 'warm'],
      [8, 4, 'warm'],
    ],
  )
  assert.equal(budgets.trials, 3)
})

test('persists every responsible stage before rejecting regressions', async () => {
  const root = await createWorkspace()
  const output = join(root, 'benchmarks/current.json')

  try {
    await assert.rejects(
      runProductionPerformance({
        executeStage: async stage => ({
          eventLoopLagMs: stage.runtime === 'native' ? 21 : 0,
          latencyMs: stage.runtime === 'native' ? 101 : 50,
          maxRssMiB: stage.runtime === 'wasm' ? 129 : 32,
          outputBytes: 123,
        }),
        generatedAt: '2026-07-28T00:00:00.000Z',
        output,
        root,
      }),
      error => {
        assert.match(error.message, /native:inspect:test-font latency/u)
        assert.match(error.message, /native:inspect:test-font event loop lag/u)
        assert.match(error.message, /wasm:inspect:test-font memory/u)
        return true
      },
    )

    const report = JSON.parse(await readFile(output, 'utf8'))

    assert.equal(report.status, 'failed')
    assert.deepEqual(
      report.stages.map(stage => stage.status),
      ['failed', 'failed', 'passed'],
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('publishes the production report from the benchmark gate', async () => {
  const [packageManifest, workflow, ignore] = await Promise.all([
    readFile(new URL('../package.json', import.meta.url), 'utf8').then(
      JSON.parse,
    ),
    readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'),
    readFile(new URL('../.gitignore', import.meta.url), 'utf8'),
  ])

  assert.equal(
    packageManifest.scripts['bench:production'],
    'pnpm run fixtures:production:conformance && cargo build --release --locked -p fontmin_app && node scripts/production-performance.mjs --output benchmarks/production-current.json',
  )
  assert.match(workflow, /run: pnpm run bench:production/u)
  assert.match(workflow, /benchmarks\/production-current\.json/u)
  assert.match(ignore, /^benchmarks\/production-current\.json$/mu)
})
