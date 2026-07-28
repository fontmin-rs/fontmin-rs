import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'
import {
  checkCargoVersionBumpSafety,
  checkReleaseCargoWorkspaces,
} from './check-release-cargo-workspaces.mjs'

const cargoManifestWithDependency = dependencyManifest => `[workspace.package]
version = "0.1.1"

[workspace.dependencies]
${dependencyManifest}
`

test('runs release Cargo checks without shell operators', async () => {
  const config = await readFile(
    new URL('../bump.config.ts', import.meta.url),
    'utf8',
  )

  assert.match(
    config,
    /execute: 'node scripts\/check-release-cargo-workspaces\.mjs'/u,
  )
  assert.doesNotMatch(config, /execute:[^\n]*&&/u)
})

test('checks the root and independent Fuzz workspaces', () => {
  const calls = []

  checkReleaseCargoWorkspaces((command, cargoArguments, options) => {
    calls.push({ cargoArguments, command, options })
  })

  assert.deepEqual(calls, [
    {
      cargoArguments: ['check', '--workspace'],
      command: 'cargo',
      options: { stdio: 'inherit' },
    },
    {
      cargoArguments: ['check', '--manifest-path', 'fuzz/Cargo.toml'],
      command: 'cargo',
      options: { stdio: 'inherit' },
    },
  ])
})

test('rejects external dependency requirements that a release bump would rewrite', () => {
  assert.doesNotThrow(() =>
    checkCargoVersionBumpSafety(
      cargoManifestWithDependency('font-subset = "0.1"'),
    ),
  )
  assert.throws(
    () =>
      checkCargoVersionBumpSafety(
        cargoManifestWithDependency('font-subset = "0.1.1"'),
      ),
    /font-subset/u,
  )
})
