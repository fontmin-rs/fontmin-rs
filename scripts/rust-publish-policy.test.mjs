import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import test from 'node:test'
import { fileURLToPath } from 'node:url'

const repositoryRoot = fileURLToPath(new URL('..', import.meta.url))
const workspacePublishPattern =
  /\[workspace\.package\][\s\S]*?\r?\npublish = false(?:\r?\n|$)/u
const inheritedPublishPattern = /^publish\.workspace = true\r?$/mu

test('recognizes publish policies with LF and CRLF line endings', () => {
  for (const newline of ['\n', '\r\n']) {
    assert.match(
      `[workspace.package]${newline}publish = false${newline}`,
      workspacePublishPattern,
    )
    assert.match(`publish.workspace = true${newline}`, inheritedPublishPattern)
  }
})

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

  assert.match(rootManifest, workspacePublishPattern)
  assert.ok(packages.length > 0)

  for (const pkg of packages) {
    assert.deepEqual(pkg.publish, [], `${pkg.name} is publishable`)
    assert.match(
      readFileSync(pkg.manifest_path, 'utf8'),
      inheritedPublishPattern,
      `${pkg.name} does not inherit the workspace publish policy`,
    )
  }
})
