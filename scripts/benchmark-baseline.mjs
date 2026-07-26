import { execFile } from 'node:child_process'
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { arch, cpus, platform, release, tmpdir } from 'node:os'
import { dirname, join, posix, relative, resolve, sep } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)
const compatibilityCandidate = 'fontmin-rs glyph + ttf2woff'
const compatibilityReference = 'fontmin glyph + ttf2woff'
const maximumCompatibilityMeanRatio = 1.1

function round(value) {
  return Number(value.toFixed(4))
}

function median(values) {
  const sorted = [...values].sort((left, right) => left - right)
  const midpoint = Math.floor(sorted.length / 2)

  return sorted.length % 2 === 0
    ? (sorted[midpoint - 1] + sorted[midpoint]) / 2
    : sorted[midpoint]
}

export function normalizeBenchmarkReport(
  report,
  { environment, fixture, generatedAt, root, version },
) {
  const benchmarks = report.files.flatMap(file =>
    file.groups.flatMap(group =>
      group.benchmarks.map(benchmark => ({
        file: relative(root, file.filepath).split(sep).join(posix.sep),
        group: group.fullName,
        hz: round(benchmark.hz),
        meanMs: round(benchmark.mean),
        name: benchmark.name,
        p75Ms: round(benchmark.p75),
        p99Ms: round(benchmark.p99),
        rmePercent: round(benchmark.rme),
        samples: benchmark.sampleCount,
      })),
    ),
  )

  return {
    schemaVersion: 1,
    generatedAt,
    version,
    environment,
    fixture,
    benchmarks,
  }
}

export function aggregateBenchmarkReports(reports) {
  if (reports.length === 0) {
    throw new Error('at least one benchmark trial is required')
  }

  const [firstReport] = reports
  const benchmarkKeys = firstReport.benchmarks.map(
    benchmark => `${benchmark.file}\0${benchmark.group}\0${benchmark.name}`,
  )

  for (const report of reports.slice(1)) {
    const keys = report.benchmarks.map(
      benchmark => `${benchmark.file}\0${benchmark.group}\0${benchmark.name}`,
    )

    if (
      keys.length !== benchmarkKeys.length ||
      keys.some((key, index) => key !== benchmarkKeys[index])
    ) {
      throw new Error('benchmark trials do not contain the same cases')
    }
  }

  const benchmarks = firstReport.benchmarks.map((benchmark, index) => {
    const trials = reports.map(report => report.benchmarks[index])

    return {
      ...benchmark,
      hz: round(median(trials.map(trial => trial.hz))),
      meanMs: round(median(trials.map(trial => trial.meanMs))),
      p75Ms: round(median(trials.map(trial => trial.p75Ms))),
      p99Ms: round(median(trials.map(trial => trial.p99Ms))),
      rmePercent: round(median(trials.map(trial => trial.rmePercent))),
      samples: trials.reduce((total, trial) => total + trial.samples, 0),
      trialMeanMs: trials.map(trial => trial.meanMs),
    }
  })
  const candidate = benchmarks.find(
    benchmark => benchmark.name === compatibilityCandidate,
  )
  const reference = benchmarks.find(
    benchmark => benchmark.name === compatibilityReference,
  )

  if (candidate === undefined || reference === undefined) {
    throw new Error('compatibility pipeline benchmarks are missing')
  }
  const meanRatio = round(candidate.meanMs / reference.meanMs)

  return {
    ...firstReport,
    schemaVersion: 2,
    trialCount: reports.length,
    benchmarks,
    comparisons: [
      {
        name: 'compatibility glyph + ttf2woff parity',
        candidate: compatibilityCandidate,
        reference: compatibilityReference,
        meanRatio,
        maximumMeanRatio: maximumCompatibilityMeanRatio,
        status:
          meanRatio <= maximumCompatibilityMeanRatio ? 'passed' : 'failed',
      },
    ],
  }
}

function parseArguments(arguments_) {
  const parsed = { output: undefined, profile: undefined, trials: undefined }

  for (let index = 0; index < arguments_.length; index += 2) {
    const flag = arguments_[index]
    const value = arguments_[index + 1]

    if (value === undefined) {
      throw new Error(`missing value for benchmark argument ${flag}`)
    }
    if (flag === '--output') {
      parsed.output = resolve(workspaceRoot, value)
    } else if (flag === '--profile') {
      parsed.profile = value
    } else if (flag === '--trials') {
      parsed.trials = Number(value)
    } else {
      throw new Error(`unexpected benchmark argument: ${flag}`)
    }
  }

  if (
    parsed.output === undefined ||
    parsed.profile === undefined ||
    parsed.trials === undefined
  ) {
    throw new Error(
      'usage: benchmark-baseline.mjs --output <path> --profile <debug|release> --trials <count>',
    )
  }
  if (parsed.profile !== 'debug' && parsed.profile !== 'release') {
    throw new Error(`unsupported binding profile: ${parsed.profile}`)
  }
  if (!Number.isSafeInteger(parsed.trials) || parsed.trials < 1) {
    throw new Error(`benchmark trial count must be a positive integer`)
  }

  return parsed
}

async function runTrial(rawReportPath) {
  const { stderr, stdout } = await executeFile(
    'pnpm',
    [
      '--filter',
      'fontmin-rs',
      'exec',
      'vitest',
      'bench',
      '--run',
      'bench',
      '--outputJson',
      rawReportPath,
    ],
    { cwd: workspaceRoot, maxBuffer: 10 * 1024 * 1024 },
  )

  if (stdout.length > 0) {
    console.log(stdout.trimEnd())
  }
  if (stderr.length > 0) {
    console.error(stderr.trimEnd())
  }

  return readFile(rawReportPath, 'utf8').then(contents => JSON.parse(contents))
}

async function run() {
  const { output, profile, trials } = parseArguments(process.argv.slice(2))
  const temporaryDirectory = await mkdtemp(join(tmpdir(), 'fontmin-bench-'))

  try {
    const rawReports = []

    for (let index = 0; index < trials; index += 1) {
      console.log(`Benchmark trial ${index + 1}/${trials}`)
      rawReports.push(
        await runTrial(join(temporaryDirectory, `vitest-${index + 1}.json`)),
      )
    }

    const [packageManifest, fixtureManifest] = await Promise.all([
      readFile(
        join(workspaceRoot, 'packages/fontmin/package.json'),
        'utf8',
      ).then(contents => JSON.parse(contents)),
      readFile(
        join(workspaceRoot, 'fixtures/fonts/manifest.json'),
        'utf8',
      ).then(contents => JSON.parse(contents)),
    ])
    const fixture = fixtureManifest.fonts.find(
      font => font.path === 'fixtures/fonts/ttf/roboto-regular.ttf',
    )

    if (fixture === undefined) {
      throw new Error('Roboto benchmark fixture is missing from the manifest')
    }

    const generatedAt = new Date().toISOString()
    const metadata = {
      environment: {
        arch: arch(),
        bindingProfile: profile,
        cpu: cpus()[0]?.model ?? 'unknown',
        node: process.version,
        os: platform(),
        osRelease: release(),
      },
      fixture: { path: fixture.path, sha256: fixture.sha256 },
      generatedAt,
      root: workspaceRoot,
      version: packageManifest.version,
    }
    const normalizedTrials = rawReports.map(report =>
      normalizeBenchmarkReport(report, metadata),
    )
    const normalizedReport = aggregateBenchmarkReports(normalizedTrials)

    await mkdir(dirname(output), { recursive: true })
    await writeFile(output, `${JSON.stringify(normalizedReport, null, 2)}\n`)
    console.log(`Normalized benchmark report written to ${output}`)

    const failedComparison = normalizedReport.comparisons.find(
      comparison => comparison.status === 'failed',
    )

    if (failedComparison !== undefined) {
      throw new Error(
        `${failedComparison.name} mean ratio ${failedComparison.meanRatio} exceeds ${failedComparison.maximumMeanRatio}`,
      )
    }
  } finally {
    await rm(temporaryDirectory, { force: true, recursive: true })
  }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  await run()
}
