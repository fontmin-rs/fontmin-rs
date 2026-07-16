import assert from 'node:assert/strict'
import test from 'node:test'
import { distTagForVersion } from './npm-dist-tag.mjs'

test('uses latest for a stable release', () => {
  assert.equal(distTagForVersion('1.2.3'), 'latest')
})

test('uses the prerelease channel as the dist-tag', () => {
  assert.equal(distTagForVersion('0.1.0-beta.1'), 'beta')
  assert.equal(distTagForVersion('2.0.0-rc.2'), 'rc')
})

test('falls back to next for a numeric prerelease channel', () => {
  assert.equal(distTagForVersion('1.0.0-0'), 'next')
})

test('rejects an invalid release version', () => {
  assert.throws(() => distTagForVersion('v1.2.3'), /invalid release version/u)
})
