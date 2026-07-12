import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

test('keeps WASM required and makes the native binding optional', async () => {
  const manifest = JSON.parse(
    await readFile(
      new URL('../packages/fontmin/package.json', import.meta.url),
      'utf8',
    ),
  )

  assert.equal(manifest.dependencies['@fontmin-rs/wasm'], 'workspace:*')
  assert.equal(manifest.dependencies['@fontmin-rs/binding'], undefined)
  assert.equal(
    manifest.optionalDependencies['@fontmin-rs/binding'],
    'workspace:*',
  )
})

test('prepares an installed consumer for auto fallback', async () => {
  const consumerDir = await mkdtemp(join(tmpdir(), 'fontmin-bindings-'))
  const nodeModules = join(consumerDir, 'node_modules')
  const bindingDir = join(nodeModules, '@fontmin-rs', 'binding')
  const platformDir = join(nodeModules, '@fontmin-rs', 'binding-darwin-arm64')
  const wasmDir = join(nodeModules, '@fontmin-rs', 'wasm')
  const mainDir = join(nodeModules, 'fontmin-rs')

  try {
    await mkdir(join(bindingDir, 'nested'), { recursive: true })
    await mkdir(platformDir, { recursive: true })
    await mkdir(wasmDir, { recursive: true })
    await mkdir(mainDir, { recursive: true })
    await writeFile(join(bindingDir, 'index.js'), 'export {}')
    await writeFile(join(bindingDir, 'fontmin.node'), 'native')
    await writeFile(join(bindingDir, 'nested', 'fontmin.node'), 'native')
    await writeFile(join(platformDir, 'package.json'), '{}')
    await writeFile(join(wasmDir, 'package.json'), '{}')
    await writeFile(join(mainDir, 'package.json'), '{}')

    const { prepareAutoFallbackConsumer } = await import('./package-smoke.mjs')

    assert.equal(typeof prepareAutoFallbackConsumer, 'function')
    await prepareAutoFallbackConsumer(consumerDir)

    assert.equal(existsSync(join(bindingDir, 'index.js')), true)
    assert.equal(existsSync(join(bindingDir, 'fontmin.node')), false)
    assert.equal(existsSync(join(bindingDir, 'nested', 'fontmin.node')), false)
    assert.equal(existsSync(platformDir), false)
    assert.equal(existsSync(join(wasmDir, 'package.json')), true)
    assert.equal(existsSync(join(mainDir, 'package.json')), true)
  } finally {
    await rm(consumerDir, { force: true, recursive: true })
  }
})

test('isolates auto fallback from installed native artifacts', async () => {
  const script = await readFile(
    new URL('package-smoke.mjs', import.meta.url),
    'utf8',
  )
  const isolatedConsumer = script.match(
    /await runConsumer\(\s*(?<tarballs>\[[^\]]+\]),\s*`(?<source>[\s\S]*?)`,\s*\[[\s\S]*?\],\s*prepareAutoFallbackConsumer,\s*\)/u,
  )

  assert.ok(isolatedConsumer, 'expected an isolated auto fallback consumer')
  const source = isolatedConsumer.groups?.source ?? ''
  const tarballs = isolatedConsumer.groups?.tarballs ?? ''

  assert.equal(tarballs, '[wasmTarball, nodeTarball]')
  assert.doesNotMatch(tarballs, /bindingTarball/u)
  assert.match(source, /NativeBindingLoadError/u)
  assert.match(source, /inspect\(new Uint8Array\(\)\)/u)
  assert.match(source, /runtime:\s*'auto'/u)
  assert.match(source, /modernWeb\(\{ text:\s*'Hello' \}\)/u)
  assert.doesNotMatch(source, /clone:\s*false/u)
})
