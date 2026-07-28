import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const workflows = await Promise.all(
  [
    ['CI', '.github/workflows/ci.yml', 4],
    ['release', '.github/workflows/release.yml', 2],
    ['Pages', '.github/workflows/build-pages.yml', 1],
  ].map(async ([name, path, installerCount]) => ({
    installerCount,
    name,
    source: await readFile(path, 'utf8'),
  })),
)
const wasmManifest = await readFile('wasm/fontmin-core/Cargo.toml', 'utf8')

test('pins a current wasm-pack installer in every publishing workflow', () => {
  for (const { installerCount, name, source } of workflows) {
    assert.doesNotMatch(source, /jetli\/wasm-pack-action/u, name)
    assert.equal(
      source.match(/tool: wasm-pack@0\.15\.0/gu)?.length,
      installerCount,
      name,
    )
  }
})

test('enables the standardized features emitted by the pinned Rust toolchain', () => {
  assert.match(
    wasmManifest,
    /\[package\.metadata\.wasm-pack\.profile\.release\][\s\S]*wasm-opt\s*=\s*\[[\s\S]*"--enable-bulk-memory"[\s\S]*"--enable-nontrapping-float-to-int"[\s\S]*\]/u,
  )
})
