import { execFile } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { arch, cpus, platform, release } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)
const budgetRelativePath = 'benchmarks/production-budgets.json'
const defaultOutputRelativePath = 'benchmarks/production-current.json'

function round(value) {
  return Number(value.toFixed(2))
}

function median(values) {
  const sorted = [...values].toSorted((left, right) => left - right)
  const midpoint = Math.floor(sorted.length / 2)

  return sorted.length % 2 === 0
    ? (sorted[midpoint - 1] + sorted[midpoint]) / 2
    : sorted[midpoint]
}

function assertBudgets(budgets) {
  if (
    budgets.schemaVersion !== 1 ||
    typeof budgets.profile !== 'string' ||
    !Number.isSafeInteger(budgets.trials) ||
    budgets.trials <= 0 ||
    !Array.isArray(budgets.stages) ||
    budgets.stages.length === 0
  ) {
    throw new Error(`${budgetRelativePath} must use schema version 1`)
  }

  const names = new Set()
  for (const stage of budgets.stages) {
    if (
      typeof stage.name !== 'string' ||
      stage.name.length === 0 ||
      names.has(stage.name) ||
      !['native', 'rust-cli', 'wasm'].includes(stage.runtime) ||
      ![
        'async-woff2',
        'auto-delivery',
        'build-scale',
        'cache-scale',
        'init',
        'inspect',
        'mixed-delivery',
        'multi-file',
      ].includes(stage.operation) ||
      (stage.entryCount !== undefined &&
        (!Number.isSafeInteger(stage.entryCount) ||
          stage.entryCount <= 0 ||
          stage.entryCount > 10_000)) ||
      (stage.inputCount !== undefined &&
        (!Number.isSafeInteger(stage.inputCount) ||
          stage.inputCount <= 0 ||
          stage.inputCount > 10_000)) ||
      (stage.inputCopies !== undefined &&
        (!Number.isSafeInteger(stage.inputCopies) ||
          stage.inputCopies <= 0 ||
          stage.inputCopies > 32)) ||
      (stage.concurrency !== undefined &&
        (!Number.isSafeInteger(stage.concurrency) ||
          stage.concurrency <= 0 ||
          stage.concurrency > 32)) ||
      (stage.threads !== undefined &&
        (!Number.isSafeInteger(stage.threads) ||
          stage.threads <= 0 ||
          stage.threads > 32)) ||
      (stage.maxEventLoopLagMs !== undefined &&
        (!Number.isFinite(stage.maxEventLoopLagMs) ||
          stage.maxEventLoopLagMs <= 0)) ||
      !Number.isFinite(stage.maxLatencyMs) ||
      stage.maxLatencyMs <= 0 ||
      !Number.isFinite(stage.maxRssMiB) ||
      stage.maxRssMiB <= 0
    ) {
      throw new Error(`${budgetRelativePath} contains an invalid stage`)
    }
    if (
      !['build-scale', 'cache-scale', 'init'].includes(stage.operation) &&
      (typeof stage.fixtureId !== 'string' || stage.fixtureId.length === 0)
    ) {
      throw new Error(`${stage.name} must declare a fixture id`)
    }
    if (
      stage.operation === 'build-scale' &&
      (stage.runtime !== 'rust-cli' ||
        stage.inputCount === undefined ||
        stage.threads === undefined)
    ) {
      throw new Error(
        `${stage.name} must declare rust-cli inputCount and threads`,
      )
    }

    names.add(stage.name)
  }
}

function stageParameters(stage) {
  return Object.fromEntries(
    ['concurrency', 'entryCount', 'inputCopies', 'inputCount', 'threads']
      .filter(name => stage[name] !== undefined)
      .map(name => [name, stage[name]]),
  )
}

function aggregateMeasurements(measurements) {
  const outputBytes = measurements[0]?.outputBytes
  const eventLoopLagMeasurements = measurements.map(
    measurement => measurement.eventLoopLagMs,
  )
  const hasEventLoopLag = eventLoopLagMeasurements.some(
    measurement => measurement !== undefined,
  )

  if (
    outputBytes === undefined ||
    measurements.some(measurement => measurement.outputBytes !== outputBytes)
  ) {
    throw new Error('production stage output changed between trials')
  }

  if (
    hasEventLoopLag &&
    eventLoopLagMeasurements.some(measurement => !Number.isFinite(measurement))
  ) {
    throw new Error(
      'production stage event loop lag changed availability between trials',
    )
  }

  return {
    ...(hasEventLoopLag
      ? {
          eventLoopLagMs: Math.max(...eventLoopLagMeasurements),
          trialEventLoopLagMs: eventLoopLagMeasurements.map(measurement =>
            round(measurement),
          ),
        }
      : {}),
    latencyMs: median(measurements.map(measurement => measurement.latencyMs)),
    maxRssMiB: Math.max(
      ...measurements.map(measurement => measurement.maxRssMiB),
    ),
    outputBytes,
    trialLatencyMs: measurements.map(measurement =>
      round(measurement.latencyMs),
    ),
    trialMaxRssMiB: measurements.map(measurement =>
      round(measurement.maxRssMiB),
    ),
  }
}

async function executeStageProcess(stage, root) {
  const timeout = Math.max(10_000, stage.maxLatencyMs * 4)
  const { stdout } = await executeFile(
    process.execPath,
    [join(root, 'scripts/production-performance-worker.mjs'), stage.name],
    {
      cwd: root,
      maxBuffer: 1024 * 1024,
      timeout,
    },
  )

  return JSON.parse(stdout.trim())
}

function evaluateStage(stage, measurement) {
  const violations = []
  const latencyMs = round(measurement.latencyMs)
  const maxRssMiB = round(measurement.maxRssMiB)
  const eventLoopLagMs =
    measurement.eventLoopLagMs === undefined
      ? undefined
      : round(measurement.eventLoopLagMs)

  if (latencyMs > stage.maxLatencyMs) {
    violations.push(
      `${stage.name} latency ${latencyMs} ms exceeds ${stage.maxLatencyMs} ms`,
    )
  }
  if (maxRssMiB > stage.maxRssMiB) {
    violations.push(
      `${stage.name} memory ${maxRssMiB} MiB exceeds ${stage.maxRssMiB} MiB`,
    )
  }
  if (
    stage.maxEventLoopLagMs !== undefined &&
    (eventLoopLagMs === undefined || eventLoopLagMs > stage.maxEventLoopLagMs)
  ) {
    violations.push(
      eventLoopLagMs === undefined
        ? `${stage.name} did not report event loop lag`
        : `${stage.name} event loop lag ${eventLoopLagMs} ms exceeds ${stage.maxEventLoopLagMs} ms`,
    )
  }

  return {
    budget: {
      ...(stage.maxEventLoopLagMs === undefined
        ? {}
        : { maxEventLoopLagMs: stage.maxEventLoopLagMs }),
      maxLatencyMs: stage.maxLatencyMs,
      maxRssMiB: stage.maxRssMiB,
    },
    ...(stage.fixtureId === undefined ? {} : { fixtureId: stage.fixtureId }),
    metrics: {
      ...measurement,
      ...(eventLoopLagMs === undefined ? {} : { eventLoopLagMs }),
      latencyMs,
      maxRssMiB,
    },
    name: stage.name,
    operation: stage.operation,
    parameters: stageParameters(stage),
    runtime: stage.runtime,
    status: violations.length === 0 ? 'passed' : 'failed',
    violations,
  }
}

function failedStage(stage, error) {
  const message = error instanceof Error ? error.message : String(error)

  return {
    budget: {
      maxLatencyMs: stage.maxLatencyMs,
      maxRssMiB: stage.maxRssMiB,
    },
    ...(stage.fixtureId === undefined ? {} : { fixtureId: stage.fixtureId }),
    metrics: null,
    name: stage.name,
    operation: stage.operation,
    parameters: stageParameters(stage),
    runtime: stage.runtime,
    status: 'failed',
    violations: [`${stage.name} execution failed: ${message}`],
  }
}

export async function runProductionPerformance({
  executeStage = executeStageProcess,
  generatedAt = new Date().toISOString(),
  output = join(workspaceRoot, defaultOutputRelativePath),
  root = workspaceRoot,
} = {}) {
  const [budgets, packageManifest] = await Promise.all([
    readFile(join(root, budgetRelativePath), 'utf8').then(JSON.parse),
    readFile(join(root, 'package.json'), 'utf8').then(JSON.parse),
  ])

  assertBudgets(budgets)

  const stages = []
  for (const stage of budgets.stages) {
    try {
      const measurements = []

      for (let trial = 0; trial < budgets.trials; trial += 1) {
        measurements.push(await executeStage(stage, root))
      }

      stages.push(evaluateStage(stage, aggregateMeasurements(measurements)))
    } catch (error) {
      stages.push(failedStage(stage, error))
    }
  }

  const violations = stages.flatMap(stage => stage.violations)
  const report = {
    environment: {
      arch: arch(),
      cpu: cpus()[0]?.model ?? 'unknown',
      node: process.version,
      os: platform(),
      osRelease: release(),
      profile: budgets.profile,
    },
    generatedAt,
    schemaVersion: 1,
    stages,
    status: violations.length === 0 ? 'passed' : 'failed',
    version: packageManifest.version,
  }

  await mkdir(dirname(output), { recursive: true })
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`)

  if (violations.length > 0) {
    throw new Error(
      `production performance budgets failed:\n${violations.join('\n')}`,
    )
  }

  return report
}

function parseOutput(arguments_) {
  if (arguments_.length === 0) {
    return join(workspaceRoot, defaultOutputRelativePath)
  }
  if (arguments_.length !== 2 || arguments_[0] !== '--output') {
    throw new Error(
      'usage: production-performance.mjs [--output <report.json>]',
    )
  }

  return resolve(workspaceRoot, arguments_[1])
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const output = parseOutput(process.argv.slice(2))
  const report = await runProductionPerformance({ output })

  console.log(
    `Production performance budgets passed for ${report.stages.length} stages; report written to ${output}.`,
  )
}
