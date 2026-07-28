import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const repositoryRoot = fileURLToPath(new URL('..', import.meta.url))

test('keeps every Rust workspace package internal-only', () => {
  const rootManifest = readFileSync(
    new URL('../Cargo.toml', import.meta.url),
    'utf8',
  )
  const metadata = JSON.parse(
    execFileSync(
      'cargo',
      ['metadata', '--locked', '--no-deps', '--format-version', '1'],
      {
        cwd: repositoryRoot,
        encoding: 'utf8',
      },
    ),
  )
  const workspaceMembers = new Set(metadata.workspace_members)
  const packages = metadata.packages.filter(pkg => workspaceMembers.has(pkg.id))

  assert.match(
    rootManifest,
    /\[workspace\.package\][\s\S]*?\npublish = false(?:\n|$)/u,
  )
  assert.ok(packages.length > 0)

  for (const pkg of packages) {
    assert.deepEqual(pkg.publish, [], `${pkg.name} is publishable`)
    assert.match(
      readFileSync(pkg.manifest_path, 'utf8'),
      /^publish\.workspace = true$/mu,
      `${pkg.name} does not inherit the workspace publish policy`,
    )
  }
})
