import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

test('removes installed native binding artifacts', async () => {
  const nodeModules = await mkdtemp(join(tmpdir(), 'fontmin-bindings-'))
  const bindingDir = join(nodeModules, '@fontmin-rs', 'binding')
  const platformDir = join(nodeModules, '@fontmin-rs', 'binding-darwin-arm64')

  try {
    await mkdir(join(bindingDir, 'nested'), { recursive: true })
    await mkdir(platformDir, { recursive: true })
    await writeFile(join(bindingDir, 'index.js'), 'export {}')
    await writeFile(join(bindingDir, 'fontmin.node'), 'native')
    await writeFile(join(bindingDir, 'nested', 'fontmin.node'), 'native')
    await writeFile(join(platformDir, 'package.json'), '{}')

    const { removeNativeArtifacts } = await import('./package-smoke.mjs')

    assert.equal(typeof removeNativeArtifacts, 'function')
    await removeNativeArtifacts(nodeModules)

    assert.equal(existsSync(join(bindingDir, 'index.js')), true)
    assert.equal(existsSync(join(bindingDir, 'fontmin.node')), false)
    assert.equal(existsSync(join(bindingDir, 'nested', 'fontmin.node')), false)
    assert.equal(existsSync(platformDir), false)
  } finally {
    await rm(nodeModules, { force: true, recursive: true })
  }
})

test('isolates auto fallback from installed native artifacts', async () => {
  const script = await readFile(
    new URL('package-smoke.mjs', import.meta.url),
    'utf8',
  )
  const isolatedConsumer = script.match(
    /await runConsumer\(\s*(?<tarballs>\[bindingTarball, wasmTarball, nodeTarball\]),\s*`(?<source>[\s\S]*?)`,/u,
  )

  assert.ok(isolatedConsumer, 'expected an isolated auto fallback consumer')
  const source = isolatedConsumer.groups?.source ?? ''

  assert.match(source, /runtime:\s*'auto'/u)
  assert.match(source, /modernWeb\(\{ text:\s*'Hello' \}\)/u)
  assert.doesNotMatch(source, /clone:\s*false/u)
  assert.match(script, /removeNativeArtifacts/u)
})
