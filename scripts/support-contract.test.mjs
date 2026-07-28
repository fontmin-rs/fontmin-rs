import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const workspaceRoot = new URL('../', import.meta.url)

async function readJson(path) {
  return JSON.parse(await readFile(new URL(path, workspaceRoot), 'utf8'))
}

test('reconfirms the 1.0 runtime and platform support matrix', async () => {
  const [support, publicContract, nodeManifest, cargo, toolchain, ci, release] =
    await Promise.all([
      readJson('contracts/support.json'),
      readJson('contracts/public-api.json'),
      readJson('packages/fontmin/package.json'),
      readFile(new URL('Cargo.toml', workspaceRoot), 'utf8'),
      readFile(new URL('rust-toolchain.toml', workspaceRoot), 'utf8'),
      readFile(new URL('.github/workflows/ci.yml', workspaceRoot), 'utf8'),
      readFile(new URL('.github/workflows/release.yml', workspaceRoot), 'utf8'),
    ])

  assert.equal(support.schemaVersion, 1)
  assert.equal(support.targetRelease, '1.0.0')
  assert.equal(support.node.engine, '>=22.18.0')
  assert.equal(nodeManifest.engines.node, support.node.engine)
  assert.deepEqual(support.node.testedMajors, [22, 24, 26])
  assert.match(ci, /node: \[22\.18\.0, 24\.x, 26\.x\]/u)
  assert.deepEqual(
    support.node.runtime.values,
    publicContract.config.node.runtimeValues,
  )
  assert.equal(support.node.runtime.default, 'native')
  assert.equal(support.node.runtime.autoFallback, 'native-load-only')

  assert.equal(support.native.nodeApi, 8)
  assert.match(cargo, /features = \["napi8", "serde-json", "tokio_rt"\]/u)
  assert.equal(support.native.targets.length, 8)
  for (const target of support.native.targets) {
    assert.match(ci, new RegExp(target, 'u'))
    assert.match(release, new RegExp(target, 'u'))
  }

  assert.deepEqual(support.browser.engines, ['chromium', 'firefox', 'webkit'])
  assert.match(ci, /browser: \[chromium, firefox, webkit\]/u)
  assert.deepEqual(support.browser.excludedCapabilities, [
    'filesystem-paths',
    'glob-expansion',
    'disk-cache',
    'cli',
    'node-plugin-hooks',
  ])

  assert.match(cargo, new RegExp(`rust-version = "${support.rust.msrv}"`, 'u'))
  assert.match(
    toolchain,
    new RegExp(`channel = "${support.rust.pinnedToolchain}"`, 'u'),
  )
})

test('keeps diagnostics, generated names, and deprecation decisions explicit', async () => {
  const [support, publicContract, nodeTypes, plugins, migration, deprecation] =
    await Promise.all([
      readJson('contracts/support.json'),
      readJson('contracts/public-api.json'),
      readFile(new URL('packages/fontmin/src/types.ts', workspaceRoot), 'utf8'),
      readFile(
        new URL('packages/fontmin/src/plugins.ts', workspaceRoot),
        'utf8',
      ),
      readFile(new URL('docs/guide/migration.md', workspaceRoot), 'utf8'),
      readFile(new URL('docs/deprecation.md', workspaceRoot), 'utf8'),
    ])

  assert.equal(
    support.publicContract.schemaVersion,
    publicContract.schemaVersion,
  )
  assert.equal(
    support.publicContract.diagnostics,
    'contracts/public-api.json#diagnosticCodes',
  )
  assert.equal(
    support.publicContract.fileNaming,
    'contracts/public-api.json#fileNaming',
  )
  assert.deepEqual(support.deprecations.eligibleRemovals, [])
  assert.deepEqual(support.deprecations.retainedCompatibilityPaths, [
    'fontmin-compatible-default-export',
    'glyph-hinting-alias',
    'woff2-fallback-runtime-alias',
  ])

  assert.match(nodeTypes, /hinting\?: boolean/u)
  assert.match(plugins, /options\.preserveHinting \?\? options\.hinting/u)
  assert.match(migration, /Upgrade From 0\.3 To 1\.0/u)
  assert.match(migration, /No public API is removed or renamed/u)
  assert.match(deprecation, /No API is eligible for removal in 1\.0/u)
})
