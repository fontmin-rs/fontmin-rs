import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

test('publishes only from a version-matched tag', async () => {
  const workflow = await readFile(
    new URL('../.github/workflows/release.yml', import.meta.url),
    'utf8',
  )

  assert.doesNotMatch(workflow, /workflow_dispatch:/u)
  assert.match(
    workflow,
    /release:metadata -- --tag "\$\{\{ github\.ref_name \}\}"/u,
  )
})

test('dry-runs platform tarballs after native artifacts are assembled', async () => {
  const workflow = await readFile(
    new URL('../.github/workflows/release.yml', import.meta.url),
    'utf8',
  )
  const placement = workflow.indexOf('Place downloaded native artifacts')
  const pack = workflow.indexOf('pnpm run release:pack', placement)

  assert.notEqual(placement, -1)
  assert.ok(pack > placement)
})
