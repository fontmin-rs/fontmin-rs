import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

test('dry-runs publishing primary and every available platform package', async () => {
  const source = await readFile(
    new URL('package-dry-run.mjs', import.meta.url),
    'utf8',
  )

  assert.match(source, /primaryPackageDirectories/u)
  assert.match(source, /validateNativeReleaseLayout/u)
  assert.match(source, /nativeReleaseEntries/u)
  assert.match(source, /manifest\.files/u)
  assert.match(
    source,
    /'publish',[\s\S]*?'--dry-run',[\s\S]*?'--no-git-checks',[\s\S]*?'--tag',[\s\S]*?'dry-run'/u,
  )
  assert.doesNotMatch(source, /executeFile\('npm'/u)
})
