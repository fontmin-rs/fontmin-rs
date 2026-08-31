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

async function createWorkspace() {
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
      stages,
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
