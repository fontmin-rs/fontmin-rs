import assert from 'node:assert/strict'
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

test('records the WASM source digest before sharing CI artifacts', async () => {
  const workflow = await readFile(
    new URL('../.github/workflows/ci.yml', import.meta.url),
    'utf8',
  )

  assert.match(workflow, /- run: node scripts\/ensure-wasm\.mjs/u)
  assert.doesNotMatch(workflow, /- run: pnpm -C wasm\/fontmin run build$/mu)
})

test('runs pnpm through the shell on Windows', async () => {
  const module = await import('./ensure-wasm.mjs')

  assert.equal(typeof module.runPnpm, 'function')

  let invocation
  await module.runPnpm(['run', 'build:js'], {
    execute: async (...args) => {
      invocation = args
    },
    platform: 'win32',
  })

  assert.equal(invocation[0], 'pnpm')
  assert.deepEqual(invocation[1], ['run', 'build:js'])
  assert.equal(invocation[2].shell, true)
  assert.equal(invocation[2].env.PATH.startsWith(process.env.PATH), true)
})

test('reuses WASM artifacts while their Rust sources are unchanged', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-wasm-'))
  const artifacts = [join(root, 'module.js'), join(root, 'module_bg.wasm')]
  const source = join(root, 'source.rs')
  const sourceStamp = join(root, 'source.sha256')
  let wasmBuilds = 0
  let packageBuilds = 0

  try {
    await mkdir(root, { recursive: true })
    await writeFile(source, 'pub fn version() -> u8 { 1 }\n')

    const { ensureWasm } = await import('./ensure-wasm.mjs')
    const options = {
      artifacts,
      buildPackage: async () => {
        packageBuilds += 1
      },
      buildWasm: async () => {
        wasmBuilds += 1
        await Promise.all(
          artifacts.map(artifact => writeFile(artifact, 'artifact')),
        )
      },
      sourceRoots: [source],
      sourceStamp,
    }
    const generated = await ensureWasm(options)
    const reused = await ensureWasm(options)

    assert.equal(generated, true)
    assert.equal(reused, false)
    assert.equal(wasmBuilds, 1)
    assert.equal(packageBuilds, 2)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('generates missing WASM artifacts before building the package', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-wasm-'))
  const artifacts = [join(root, 'module.js'), join(root, 'module_bg.wasm')]
  const source = join(root, 'source.rs')
  const sourceStamp = join(root, 'source.sha256')
  let wasmBuilds = 0
  let packageBuilds = 0

  try {
    await writeFile(source, 'pub fn version() -> u8 { 1 }\n')
    const { ensureWasm } = await import('./ensure-wasm.mjs')
    const generated = await ensureWasm({
      artifacts,
      buildPackage: async () => {
        packageBuilds += 1
      },
      buildWasm: async () => {
        wasmBuilds += 1
      },
      sourceRoots: [source],
      sourceStamp,
    })

    assert.equal(generated, true)
    assert.equal(wasmBuilds, 1)
    assert.equal(packageBuilds, 1)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('regenerates WASM artifacts after a Rust source change', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-wasm-'))
  const artifacts = [join(root, 'module.js'), join(root, 'module_bg.wasm')]
  const source = join(root, 'source.rs')
  const sourceStamp = join(root, 'source.sha256')
  let wasmBuilds = 0
  let packageBuilds = 0

  try {
    await writeFile(source, 'pub fn version() -> u8 { 1 }\n')
    const { ensureWasm } = await import('./ensure-wasm.mjs')
    const options = {
      artifacts,
      buildPackage: async () => {
        packageBuilds += 1
      },
      buildWasm: async () => {
        wasmBuilds += 1
        await Promise.all(
          artifacts.map(artifact => writeFile(artifact, 'artifact')),
        )
      },
      sourceRoots: [source],
      sourceStamp,
    }

    await ensureWasm(options)
    await writeFile(source, 'pub fn version() -> u8 { 2 }\n')
    const regenerated = await ensureWasm(options)

    assert.equal(regenerated, true)
    assert.equal(wasmBuilds, 2)
    assert.equal(packageBuilds, 2)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})
