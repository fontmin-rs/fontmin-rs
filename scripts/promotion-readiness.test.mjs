import assert from 'node:assert/strict'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import {
  isVersionOnlyRuntimeDiff,
  promotionReadiness,
  validatePromotionReadiness,
} from './promotion-readiness.mjs'

async function evidence() {
  const audit = JSON.parse(
    await readFile(
      new URL('../audits/1.0.0-readiness.json', import.meta.url),
      'utf8',
    ),
  )
  const report = JSON.parse(
    await readFile(
      new URL('../compatibility/1.0.0-rc.1.json', import.meta.url),
      'utf8',
    ),
  )

  return { audit, report }
}

test('accepts the reviewed RC evidence without runtime source changes', async () => {
  const { audit, report } = await evidence()

  assert.doesNotThrow(() =>
    validatePromotionReadiness({
      audit,
      report,
      targetTag: 'v1.0.0',
    }),
  )

  const result = await promotionReadiness({
    execute: async (_command, args) => {
      if (args[0] === 'rev-list') {
        return { stdout: `${audit.candidate.commit}\n` }
      }

      assert.equal(args[0], 'diff')

      return { stdout: '' }
    },
    tag: 'v1.0.0',
  })

  assert.deepEqual(result, {
    candidate: '1.0.0-rc.1',
    runtimeChanges: 0,
    status: 'passed',
    target: '1.0.0',
    versionOnlyChanges: 0,
  })
})

test('accepts exact stable version substitutions in generated runtime constants', async () => {
  const { audit } = await evidence()
  const changedPaths = [
    'packages/fontmin/src/cli.mjs',
    'packages/fontmin/src/optimize-storage.ts',
  ]
  const result = await promotionReadiness({
    execute: async (_command, args) => {
      if (args[0] === 'rev-list') {
        return { stdout: `${audit.candidate.commit}\n` }
      }
      if (args.includes('--name-only')) {
        return { stdout: `${changedPaths.join('\n')}\n` }
      }

      const path = args.at(-1)

      return {
        stdout: [
          `--- a/${path}`,
          `+++ b/${path}`,
          '@@ -1 +1 @@',
          "-const version = '1.0.0-rc.1'",
          "+const version = '1.0.0'",
          '',
        ].join('\n'),
      }
    },
    tag: 'v1.0.0',
  })

  assert.deepEqual(result, {
    candidate: '1.0.0-rc.1',
    runtimeChanges: 0,
    status: 'passed',
    target: '1.0.0',
    versionOnlyChanges: 2,
  })
})

test('accepts a later RC to stable promotion cycle', async () => {
  const { audit, report } = await evidence()
  const laterAudit = structuredClone(audit)
  const laterReport = structuredClone(report)

  laterAudit.candidate = {
    commit: 'abc123',
    tag: 'v1.0.2-rc.1',
    version: '1.0.2-rc.1',
  }
  laterAudit.target = { tag: 'v1.0.2', version: '1.0.2' }
  laterReport.source.version = '1.0.2-rc.1'

  assert.doesNotThrow(() =>
    validatePromotionReadiness({
      audit: laterAudit,
      report: laterReport,
      targetTag: 'v1.0.2',
    }),
  )
  assert.equal(
    isVersionOnlyRuntimeDiff(
      ["-const version = '1.0.2-rc.1'", "+const version = '1.0.2'"].join('\n'),
      { fromVersion: '1.0.2-rc.1', toVersion: '1.0.2' },
    ),
    true,
  )
})

test('loads the target-specific audit selected by the stable tag', async () => {
  const { audit, report } = await evidence()
  const root = await mkdtemp(join(tmpdir(), 'fontmin-promotion-readiness-'))
  const laterAudit = structuredClone(audit)
  const laterReport = structuredClone(report)

  laterAudit.candidate = {
    commit: 'abc123',
    tag: 'v1.0.2-rc.1',
    version: '1.0.2-rc.1',
  }
  laterAudit.evidence.registryCompatibility.path =
    'compatibility/1.0.2-rc.1.json'
  laterAudit.target = { tag: 'v1.0.2', version: '1.0.2' }
  laterReport.source.version = '1.0.2-rc.1'

  try {
    await mkdir(join(root, 'audits'))
    await mkdir(join(root, 'compatibility'))
    await writeFile(
      join(root, 'audits/1.0.2-readiness.json'),
      JSON.stringify(laterAudit),
    )
    await writeFile(
      join(root, 'compatibility/1.0.2-rc.1.json'),
      JSON.stringify(laterReport),
    )

    const result = await promotionReadiness({
      execute: async (_command, args) => {
        if (args[0] === 'rev-list') {
          return { stdout: 'abc123\n' }
        }

        return { stdout: '' }
      },
      root,
      tag: 'v1.0.2',
    })

    assert.equal(result.target, '1.0.2')
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects additional edits in a version-only runtime path', () => {
  assert.equal(
    isVersionOnlyRuntimeDiff(
      [
        '--- a/packages/fontmin/src/cli.mjs',
        '+++ b/packages/fontmin/src/cli.mjs',
        '@@ -1,2 +1,2 @@',
        "-const version = '1.0.0-rc.1'",
        '-const format = true',
        "+const version = '1.0.0'",
        '+const format = false',
      ].join('\n'),
      { fromVersion: '1.0.0-rc.1', toVersion: '1.0.0' },
    ),
    false,
  )
})

test('rejects unresolved P0 or P1 issues', async () => {
  const { audit, report } = await evidence()
  const withIssue = structuredClone(audit)

  withIssue.categories.packaging.unresolvedP1.push('package mismatch')

  assert.throws(
    () =>
      validatePromotionReadiness({
        audit: withIssue,
        report,
        targetTag: 'v1.0.0',
      }),
    /packaging has P0\/P1 issues/u,
  )
})

test('rejects runtime source changes after the RC', async () => {
  const { audit, report } = await evidence()

  assert.throws(
    () =>
      validatePromotionReadiness({
        audit,
        changedRuntimePaths: ['crates/fontmin/src/lib.rs'],
        report,
        targetTag: 'v1.0.0',
      }),
    /runtime sources changed after the RC/u,
  )
})

test('rejects evidence for a different stable target', async () => {
  const { audit, report } = await evidence()

  assert.throws(
    () =>
      validatePromotionReadiness({
        audit,
        report,
        targetTag: 'v1.0.1',
      }),
    /audit targets v1\.0\.0, not v1\.0\.1/u,
  )
})

test('rejects a non-RC prerelease candidate', async () => {
  const { audit, report } = await evidence()
  const betaAudit = structuredClone(audit)
  const betaReport = structuredClone(report)

  betaAudit.candidate.tag = 'v1.0.0-beta.1'
  betaAudit.candidate.version = '1.0.0-beta.1'
  betaReport.source.version = '1.0.0-beta.1'

  assert.throws(
    () =>
      validatePromotionReadiness({
        audit: betaAudit,
        report: betaReport,
        targetTag: 'v1.0.0',
      }),
    /must reference an RC tag/u,
  )
})

test('rejects an empty registry compatibility report', async () => {
  const { audit, report } = await evidence()
  const emptyReport = structuredClone(report)

  emptyReport.summary = { failed: 0, passed: 0, total: 0 }

  assert.throws(
    () =>
      validatePromotionReadiness({
        audit,
        report: emptyReport,
        targetTag: 'v1.0.0',
      }),
    /published RC compatibility evidence is incomplete/u,
  )
})
