import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

test('dry-runs primary and every available platform package', async () => {
  const source = await readFile(
    new URL('package-dry-run.mjs', import.meta.url),
    'utf8',
  )

  assert.match(source, /primaryPackageDirectories/u)
  assert.match(source, /platformPackageDirectories/u)
  assert.match(source, /manifest\.files/u)
  assert.match(source, /executeFile\('npm', \['pack', '--dry-run'\]/u)
})
