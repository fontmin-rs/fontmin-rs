import assert from 'node:assert/strict'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import {
  auditArtifacts,
  auditDependencyPolicy,
} from './dependency-artifact-audit.mjs'

test('accounts for every duplicated Cargo dependency and vendored patch', async () => {
  const report = await auditDependencyPolicy()

  assert.deepEqual(
    report.duplicates.map(duplicate => duplicate.name),
    [
      'brotli',
      'brotli-decompressor',
      'thiserror',
      'thiserror-impl',
      'unicode-width',
    ],
  )
  assert.deepEqual(
    report.vendored.map(entry => entry.crate),
    [
      'allsorts',
      'oxifont-core',
      'oxifont-subset',
      'safer-bytes',
      'woff2-patched',
    ],
  )
  assert.equal(report.status, 'passed')
})

test('persists artifact measurements before rejecting a size regression', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-artifact-audit-'))
  const output = join(root, 'audits/current.json')

  try {
    await mkdir(join(root, 'artifacts/native'), { recursive: true })
    await mkdir(join(root, 'audits'), { recursive: true })
    await writeFile(join(root, 'package.json'), '{"version":"0.3.0"}')
    await writeFile(join(root, 'artifacts/cli'), Buffer.alloc(10))
    await writeFile(
      join(root, 'artifacts/native/fontmin.node'),
      Buffer.alloc(20),
    )
    await writeFile(join(root, 'artifacts/runtime.wasm'), Buffer.alloc(30))
    await writeFile(
      join(root, 'audits/release-policy.json'),
      JSON.stringify({
        artifacts: [
          {
            id: 'cli',
            maxBytes: 10,
            paths: ['artifacts/cli'],
          },
          {
            directory: 'artifacts/native',
            id: 'native-binding',
            maxBytes: 19,
            suffix: '.node',
          },
          {
            id: 'wasm',
            maxBytes: 30,
            paths: ['artifacts/runtime.wasm'],
          },
        ],
        duplicateDependencies: [],
        schemaVersion: 1,
        vendoredPatches: [],
      }),
    )

    await assert.rejects(
      auditArtifacts({
        generatedAt: '2026-07-28T00:00:00.000Z',
        output,
        root,
      }),
      /native-binding is 20 bytes; budget is 19 bytes/u,
    )

    const report = JSON.parse(await readFile(output, 'utf8'))

    assert.equal(report.status, 'failed')
    assert.deepEqual(
      report.artifacts.map(artifact => artifact.status),
      ['passed', 'failed', 'passed'],
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('gates dependency decisions and artifact budgets in repository workflows', async () => {
  const [ignore, packageManifest, workflow] = await Promise.all([
    readFile(new URL('../.gitignore', import.meta.url), 'utf8'),
    readFile(new URL('../package.json', import.meta.url), 'utf8').then(
      JSON.parse,
    ),
    readFile(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'),
  ])

  assert.equal(
    packageManifest.scripts['audit:dependencies'],
    'node scripts/dependency-artifact-audit.mjs --dependencies-only',
  )
  assert.match(packageManifest.scripts.check, /audit:dependencies/u)
  assert.match(packageManifest.scripts['release:check'], /audit:artifacts/u)
  assert.match(workflow, /dependency-artifact-audit\.mjs --output/u)
  assert.match(workflow, /audits\/artifact-current\.json/u)
  assert.match(ignore, /^audits\/artifact-current\.json$/mu)
})
