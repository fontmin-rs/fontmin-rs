import assert from 'node:assert/strict'
import { mkdtemp, readFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { runCompatibilityReport } from './compatibility-report.mjs'

test('records every standalone consumer against workspace tarballs', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'fontmin-compatibility-'))
  const output = join(directory, 'report.json')
  const calls = []
  const scenarios = [
    {
      command: ['node', 'cli-node.mjs'],
      id: 'cli-node-project',
      interfaces: ['cli', 'node'],
    },
    {
      command: ['node', 'browser.mjs'],
      id: 'browser-project',
      interfaces: ['browser-wasm'],
    },
  ]

  try {
    const report = await runCompatibilityReport({
      browser: 'chromium',
      execute: async (file, arguments_, options) => {
        calls.push({ arguments_, file, options })
      },
      output,
      packageVersion: '0.3.0',
      scenarios,
    })

    assert.equal(report.summary.passed, 2)
    assert.equal(report.summary.total, 2)
    assert.deepEqual(
      report.cases.map(result => ({
        id: result.id,
        interfaces: result.interfaces,
        status: result.status,
      })),
      [
        {
          id: 'cli-node-project',
          interfaces: ['cli', 'node'],
          status: 'passed',
        },
        {
          id: 'browser-project',
          interfaces: ['browser-wasm'],
          status: 'passed',
        },
      ],
    )
    assert.deepEqual(report.source, {
      type: 'workspace-tarballs',
      version: '0.3.0',
    })
    assert.equal(calls[0].options.env.BROWSER, 'chromium')
    assert.equal(calls[0].options.env.FONTMIN_REGISTRY_VERSION, undefined)
    assert.deepEqual(JSON.parse(await readFile(output, 'utf8')), report)
  } finally {
    await rm(directory, { force: true, recursive: true })
  }
})

test('passes an exact registry version to every consumer', async () => {
  const calls = []
  const report = await runCompatibilityReport({
    execute: async (_file, _arguments, options) => {
      calls.push(options.env.FONTMIN_REGISTRY_VERSION)
    },
    output: undefined,
    packageVersion: '1.0.0-rc.1',
    registryVersion: '1.0.0-rc.1',
    scenarios: [
      {
        command: ['node', 'consumer.mjs'],
        id: 'registry-project',
        interfaces: ['cli'],
      },
    ],
  })

  assert.deepEqual(calls, ['1.0.0-rc.1'])
  assert.deepEqual(report.source, {
    type: 'npm-registry',
    version: '1.0.0-rc.1',
  })
})

test('writes failed consumer evidence before rejecting the gate', async () => {
  const directory = await mkdtemp(join(tmpdir(), 'fontmin-compatibility-'))
  const output = join(directory, 'report.json')

  try {
    await assert.rejects(
      runCompatibilityReport({
        execute: async () => {
          throw new Error('consumer mismatch')
        },
        output,
        packageVersion: '0.3.0',
        scenarios: [
          {
            command: ['node', 'consumer.mjs'],
            id: 'failing-project',
            interfaces: ['node'],
          },
        ],
      }),
      /compatibility project failed/u,
    )

    const report = JSON.parse(await readFile(output, 'utf8'))

    assert.equal(report.summary.failed, 1)
    assert.equal(report.cases[0].status, 'failed')
    assert.match(report.cases[0].error, /consumer mismatch/u)
  } finally {
    await rm(directory, { force: true, recursive: true })
  }
})

test('keeps compatibility evidence in CI and the release gate', async () => {
  const [manifest, ci, release] = await Promise.all([
    readFile(new URL('../package.json', import.meta.url), 'utf8').then(
      JSON.parse,
    ),
    readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'),
    readFile(
      new URL('../.github/workflows/release.yml', import.meta.url),
      'utf8',
    ),
  ])

  assert.match(
    manifest.scripts['release:check'],
    /package:smoke.*compatibility:report/u,
  )
  assert.match(ci, /pnpm run compatibility:report/u)
  assert.match(ci, /name: compatibility-report/u)
  assert.match(ci, /path: compatibility\/current\.json/u)
  assert.match(release, /pnpm run compatibility:report/u)
})
