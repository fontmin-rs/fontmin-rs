import assert from 'node:assert/strict'
import { mkdtemp, mkdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'

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
})

test('reuses existing WASM artifacts before building the package', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-wasm-'))
  const artifacts = [join(root, 'module.js'), join(root, 'module_bg.wasm')]
  let wasmBuilds = 0
  let packageBuilds = 0

  try {
    await Promise.all(
      artifacts.map(async artifact => {
        await mkdir(root, { recursive: true })
        await writeFile(artifact, 'artifact')
      }),
    )

    const { ensureWasm } = await import('./ensure-wasm.mjs')
    const generated = await ensureWasm({
      artifacts,
      buildPackage: async () => {
        packageBuilds += 1
      },
      buildWasm: async () => {
        wasmBuilds += 1
      },
    })

    assert.equal(generated, false)
    assert.equal(wasmBuilds, 0)
    assert.equal(packageBuilds, 1)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('generates missing WASM artifacts before building the package', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-wasm-'))
  const artifacts = [join(root, 'module.js'), join(root, 'module_bg.wasm')]
  let wasmBuilds = 0
  let packageBuilds = 0

  try {
    const { ensureWasm } = await import('./ensure-wasm.mjs')
    const generated = await ensureWasm({
      artifacts,
      buildPackage: async () => {
        packageBuilds += 1
      },
      buildWasm: async () => {
        wasmBuilds += 1
      },
    })

    assert.equal(generated, true)
    assert.equal(wasmBuilds, 1)
    assert.equal(packageBuilds, 1)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})
