import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'
import {
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

  const result = await promotionReadiness()

  assert.deepEqual(result, {
    candidate: '1.0.0-rc.1',
    runtimeChanges: 0,
    status: 'passed',
  })
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
