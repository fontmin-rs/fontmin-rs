import { execFile } from 'node:child_process'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)
const defaultScenarios = [
  {
    command: [
      process.execPath,
      join(workspaceRoot, 'scripts/package-smoke.mjs'),
    ],
    id: 'standalone-cli-node-project',
    interfaces: ['cli', 'node-native', 'node-wasm-fallback'],
  },
  {
    command: [
      process.execPath,
      join(workspaceRoot, 'packages/fontmin/tests/browser-load.mjs'),
    ],
    id: 'standalone-browser-font-project',
    interfaces: ['browser-css-font-load'],
  },
  {
    command: [
      process.execPath,
      join(workspaceRoot, 'wasm/fontmin/tests/browser-runtime.mjs'),
    ],
    id: 'standalone-browser-wasm-project',
    interfaces: ['browser-wasm'],
  },
]

function errorMessage(error) {
  if (error instanceof Error) {
    return error.message
  }

  return String(error)
}

async function writeReport(output, report) {
  if (output === undefined) {
    return
  }

  await mkdir(dirname(output), { recursive: true })
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`)
}

export async function runCompatibilityReport({
  browser = 'chromium',
  execute = executeFile,
  output,
  packageVersion,
  registryVersion,
  scenarios = defaultScenarios,
} = {}) {
  const source = {
    type: registryVersion === undefined ? 'workspace-tarballs' : 'npm-registry',
    version: registryVersion ?? packageVersion,
  }
  const cases = []

  for (const scenario of scenarios) {
    const [file, ...arguments_] = scenario.command

    try {
      await execute(file, arguments_, {
        cwd: workspaceRoot,
        env: {
          ...process.env,
          BROWSER: browser,
          FONTMIN_REGISTRY_VERSION: registryVersion,
        },
        maxBuffer: 20 * 1024 * 1024,
      })
      cases.push({
        id: scenario.id,
        interfaces: scenario.interfaces,
        status: 'passed',
      })
    } catch (error) {
      cases.push({
        error: errorMessage(error),
        id: scenario.id,
        interfaces: scenario.interfaces,
        status: 'failed',
      })
    }
  }

  const failed = cases.filter(result => result.status === 'failed').length
  const report = {
    schemaVersion: 1,
    source,
    environment: {
      arch: process.arch,
      browser,
      node: process.version,
      platform: process.platform,
    },
    summary: {
      failed,
      passed: cases.length - failed,
      total: cases.length,
    },
    cases,
  }

  await writeReport(output, report)

  if (failed > 0) {
    throw new Error(
      `${failed} compatibility project failed; see ${output ?? 'the report'}`,
    )
  }

  return report
}

function argumentValue(name) {
  const index = process.argv.indexOf(name)

  if (index === -1) {
    return
  }

  const value = process.argv[index + 1]

  if (value === undefined || value.startsWith('--')) {
    throw new Error(`${name} requires a value`)
  }

  return value
}

if (
  process.argv[1] !== undefined &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url
) {
  const manifest = JSON.parse(
    await readFile(join(workspaceRoot, 'package.json'), 'utf8'),
  )
  const output =
    argumentValue('--output') ??
    join(workspaceRoot, 'compatibility/current.json')
  const registryVersion = argumentValue('--registry-version')
  const browser = argumentValue('--browser') ?? 'chromium'
  const report = await runCompatibilityReport({
    browser,
    output,
    packageVersion: manifest.version,
    registryVersion,
  })

  process.stdout.write(
    `Compatibility projects passed: ${report.summary.passed}/${report.summary.total}\n`,
  )
}
