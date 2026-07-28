import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'
import {
  isVersionOnlyRuntimeDiff,
  promotionReadiness,
  validatePromotionReadiness,
} from './promotion-readiness.mjs'

async function evidence() {
  const audit = JSON.parse(
    await readFile(
      new URL('../audits/1.0-readiness.json', import.meta.url),
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

  assert.doesNotThrow(() => validatePromotionReadiness({ audit, report }))

  const result = await promotionReadiness({
    execute: async (_command, args) => {
      if (args[0] === 'rev-list') {
        return { stdout: `${audit.candidate.commit}\n` }
      }

      assert.equal(args[0], 'diff')

      return { stdout: '' }
    },
  })

  assert.deepEqual(result, {
    candidate: '1.0.0-rc.1',
    runtimeChanges: 0,
    status: 'passed',
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
  })

  assert.deepEqual(result, {
    candidate: '1.0.0-rc.1',
    runtimeChanges: 0,
    status: 'passed',
    versionOnlyChanges: 2,
  })
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
    ),
    false,
  )
})

test('rejects unresolved P0 or P1 issues', async () => {
  const { audit, report } = await evidence()
  const withIssue = structuredClone(audit)

  withIssue.categories.packaging.unresolvedP1.push('package mismatch')

  assert.throws(
    () => validatePromotionReadiness({ audit: withIssue, report }),
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
      }),
    /runtime sources changed after the RC/u,
  )
})
