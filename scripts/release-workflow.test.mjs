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

test('builds the WASM package before release typechecking', async () => {
  const workflow = await readFile(
    new URL('../.github/workflows/release.yml', import.meta.url),
    'utf8',
  )
  const check = workflow.indexOf('- run: pnpm run check')
  const wasmBuild = workflow.lastIndexOf(
    '- run: pnpm -C wasm/fontmin run build',
    check,
  )

  assert.notEqual(check, -1)
  assert.ok(wasmBuild > 0)
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

test('publishes prereleases with the resolved dist-tag and provenance', async () => {
  const workflow = await readFile(
    new URL('../.github/workflows/release.yml', import.meta.url),
    'utf8',
  )

  assert.match(workflow, /registry-url: https:\/\/registry\.npmjs\.org/u)
  assert.match(workflow, /id-token: write/u)
  assert.match(
    workflow,
    /publish .*--tag "\$\{\{ steps\.npm-dist-tag\.outputs\.value \}\}" --provenance/u,
  )
  assert.doesNotMatch(workflow, /NODE_AUTH_TOKEN|NPM_TOKEN/u)
})

test('creates the GitHub release only after npm publishing succeeds', async () => {
  const workflow = await readFile(
    new URL('../.github/workflows/release.yml', import.meta.url),
    'utf8',
  )
  const publish = workflow.indexOf('name: Publish npm packages')
  const githubRelease = workflow.indexOf('name: Create GitHub release')

  assert.notEqual(publish, -1)
  assert.ok(githubRelease > publish)
  assert.match(workflow, /node node_modules\/changelogithub\/cli\.mjs/u)
  assert.doesNotMatch(workflow, /pnpm exec changelogithub/u)
  assert.doesNotMatch(workflow, /pnpm dlx changelogithub/u)
  assert.doesNotMatch(workflow, /npm install -g npm@latest/u)
})
