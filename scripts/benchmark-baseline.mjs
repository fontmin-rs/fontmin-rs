import { execFile } from 'node:child_process'
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { arch, cpus, platform, release, tmpdir } from 'node:os'
import { dirname, join, posix, relative, resolve, sep } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)

function round(value) {
  return Number(value.toFixed(4))
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

function outputPathFromArguments(arguments_) {
  const outputIndex = arguments_.indexOf('--output')

  if (outputIndex === -1 || arguments_[outputIndex + 1] === undefined) {
    throw new Error('usage: benchmark-baseline.mjs --output <path>')
  }
  if (arguments_.length !== 2) {
    throw new Error(`unexpected benchmark arguments: ${arguments_.join(' ')}`)
  }

  return resolve(workspaceRoot, arguments_[outputIndex + 1])
}

async function run() {
  const outputPath = outputPathFromArguments(process.argv.slice(2))
  const temporaryDirectory = await mkdtemp(join(tmpdir(), 'fontmin-bench-'))
  const rawReportPath = join(temporaryDirectory, 'vitest.json')

  try {
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

    const [rawReport, packageManifest, fixtureManifest] = await Promise.all([
      readFile(rawReportPath, 'utf8').then(contents => JSON.parse(contents)),
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

    const normalizedReport = normalizeBenchmarkReport(rawReport, {
      environment: {
        arch: arch(),
        bindingProfile: 'debug',
        cpu: cpus()[0]?.model ?? 'unknown',
        node: process.version,
        os: platform(),
        osRelease: release(),
      },
      fixture: { path: fixture.path, sha256: fixture.sha256 },
      generatedAt: new Date().toISOString(),
      root: workspaceRoot,
      version: packageManifest.version,
    })

    await mkdir(dirname(outputPath), { recursive: true })
    await writeFile(
      outputPath,
      `${JSON.stringify(normalizedReport, null, 2)}\n`,
    )
    console.log(`Normalized benchmark report written to ${outputPath}`)
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
