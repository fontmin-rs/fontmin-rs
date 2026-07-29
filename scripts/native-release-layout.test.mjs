import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'
import {
  findNativeReleaseEntry,
  nativeTargetToReleaseEntry,
  readNativeReleaseLayout,
  validateNativeReleaseLayout,
} from './native-release-layout.mjs'

test('derives package and artifact names from N-API targets', () => {
  assert.deepEqual(nativeTargetToReleaseEntry('aarch64-unknown-linux-gnu'), {
    artifactName: 'fontmin_rs.linux-arm64-gnu.node',
    cpu: 'arm64',
    directory: 'npm/binding-linux-arm64-gnu',
    libc: 'glibc',
    name: '@fontmin-rs/binding-linux-arm64-gnu',
    os: 'linux',
    packageDirectory: 'binding-linux-arm64-gnu',
    platform: 'linux-arm64-gnu',
    target: 'aarch64-unknown-linux-gnu',
  })
  assert.throws(
    () => nativeTargetToReleaseEntry('armv7-linux-androideabi'),
    /unsupported native target/iu,
  )
})

test('keeps package manifests aligned with the canonical target list', async () => {
  const entries = await validateNativeReleaseLayout()

  assert.equal(entries.length, 8)
  assert.equal(
    findNativeReleaseEntry(entries, {
      arch: 'x64',
      libc: 'musl',
      platform: 'linux',
    }).artifactName,
    'fontmin_rs.linux-x64-musl.node',
  )
})

test('keeps CI and release build matrices aligned with N-API targets', async () => {
  const { entries } = await readNativeReleaseLayout()
  const expectedTargets = entries.map(entry => entry.target).sort()

  for (const workflowPath of [
    '../.github/workflows/ci.yml',
    '../.github/workflows/release.yml',
  ]) {
    const workflow = await readFile(
      new URL(workflowPath, import.meta.url),
      'utf8',
    )
    const workflowTargets = [
      ...workflow.matchAll(/^\s+target: (?<target>\S+)$/gmu),
    ]
      .map(match => match.groups.target)
      .sort()

    assert.deepEqual(workflowTargets, expectedTargets, workflowPath)
  }
})
