import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { test } from 'node:test'

test('covers auto optimize without installing the native binding tarball', async () => {
  const source = await readFile(
    new URL('package-smoke.mjs', import.meta.url),
    'utf8',
  )
  const autoConsumer = source.slice(source.indexOf('const autoOptimizeSource'))

  assert.match(autoConsumer, /runtime: 'auto'/u)
  assert.match(autoConsumer, /\[wasmTarball, nodeTarball\]/u)
  assert.doesNotMatch(
    autoConsumer,
    /\[bindingTarball, wasmTarball, nodeTarball\]/u,
  )
})

test('installs the native binding through its platform package', async () => {
  const source = await readFile(
    new URL('package-smoke.mjs', import.meta.url),
    'utf8',
  )

  assert.match(source, /const platformTarball = await packPlatformPackage/u)
  assert.match(
    source,
    /\[bindingTarball, platformTarball, wasmTarball, nodeTarball\]/u,
  )
  assert.match(source, /const info = inspect\(input\)/u)
  assert.match(source, /info\.format !== 'ttf'/u)
})
