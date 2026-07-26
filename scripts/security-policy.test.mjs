import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const repositoryFile = path =>
  readFile(new URL(`../${path}`, import.meta.url), 'utf8')

test('keeps Rust advisories exception-free', async () => {
  const [cargoManifest, cargoLock, denyPolicy, fuzzManifest, fuzzLock] =
    await Promise.all([
      repositoryFile('Cargo.toml'),
      repositoryFile('Cargo.lock'),
      repositoryFile('deny.toml'),
      repositoryFile('fuzz/Cargo.toml'),
      repositoryFile('fuzz/Cargo.lock'),
    ])

  assert.doesNotMatch(denyPolicy, /^\s*ignore\s*=/mu)
  assert.doesNotMatch(cargoLock, /^name = "(?:paste|ttf-parser)"$/mu)
  assert.doesNotMatch(fuzzLock, /^name = "(?:paste|ttf-parser)"$/mu)
  assert.match(
    cargoManifest,
    /safer-bytes = \{ path = "vendor\/safer-bytes" \}/u,
  )
  assert.match(cargoManifest, /allsorts = \{ path = "vendor\/allsorts" \}/u)
  assert.match(
    fuzzManifest,
    /safer-bytes = \{ path = "\.\.\/vendor\/safer-bytes" \}/u,
  )
  assert.match(
    fuzzManifest,
    /allsorts = \{ path = "\.\.\/vendor\/allsorts" \}/u,
  )
})

test('pins audited transitive npm replacements', async () => {
  const workspace = await repositoryFile('pnpm-workspace.yaml')

  assert.match(workspace, /brace-expansion@<=5\.0\.7: \^5\.0\.8/u)
  assert.match(workspace, /fast-xml-parser@<5\.7\.0: \^5\.7\.0/u)
  assert.match(workspace, /tar: 7\.5\.22/u)
  assert.match(workspace, /uuid@<11\.1\.1: \^11\.1\.1/u)
})

test('publishes a private vulnerability reporting path', async () => {
  const securityPolicy = await repositoryFile('SECURITY.md')

  assert.match(
    securityPolicy,
    /fontmin-rs\/fontmin-rs\/security\/advisories\/new/u,
  )
  assert.match(securityPolicy, /Do not open a public issue/u)
})
