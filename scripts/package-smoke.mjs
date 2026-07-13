import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { mkdtemp, readdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)

async function packPackage(directory, tarballDirectory) {
  await executeFile('pnpm', ['pack', '--pack-destination', tarballDirectory], {
    cwd: join(workspaceRoot, directory),
  })

  const files = await readdir(tarballDirectory)
  const tarballs = files
    .filter(fileName => fileName.endsWith('.tgz'))
    .map(fileName => join(tarballDirectory, fileName))

  assert.equal(tarballs.length, 1, `expected one tarball for ${directory}`)
  return tarballs[0]
}

async function packPlatformPackage(tarballDirectory) {
  const npmDirectory = join(workspaceRoot, 'npm')
  const entries = await readdir(npmDirectory, { withFileTypes: true })
  const candidates = []

  for (const entry of entries) {
    if (entry.isDirectory()) {
      const files = await readdir(join(npmDirectory, entry.name))
      if (files.some(fileName => fileName.endsWith('.node'))) {
        candidates.push(join('npm', entry.name))
      }
    }
  }

  assert.equal(
    candidates.length,
    1,
    `expected one local platform package, got ${candidates.join(', ') || 'none'}`,
  )

  return packPackage(candidates[0], tarballDirectory)
}

async function runConsumer(tarballs, source) {
  const directory = await mkdtemp(join(tmpdir(), 'fontmin-package-smoke-'))

  try {
    await writeFile(
      join(directory, 'package.json'),
      JSON.stringify({
        name: 'fontmin-package-smoke',
        private: true,
        type: 'module',
      }),
    )
    await executeFile('npm', ['install', '--ignore-scripts', ...tarballs], {
      cwd: directory,
    })
    await executeFile(
      process.execPath,
      ['--input-type=module', '--eval', source],
      {
        cwd: directory,
      },
    )
  } finally {
    await rm(directory, { force: true, recursive: true })
  }
}

const tarballRoot = await mkdtemp(join(tmpdir(), 'fontmin-tarballs-'))

try {
  const bindingTarball = await packPackage(
    'napi/fontmin',
    join(tarballRoot, 'binding'),
  )
  const platformTarball = await packPlatformPackage(
    join(tarballRoot, 'platform'),
  )
  const nodeTarball = await packPackage(
    'packages/fontmin',
    join(tarballRoot, 'node'),
  )
  const wasmTarball = await packPackage(
    'wasm/fontmin',
    join(tarballRoot, 'wasm'),
  )

  const nativeInspectSource = `
    import { readFile } from 'node:fs/promises'
    import { inspect } from 'fontmin-rs'

    const input = await readFile(${JSON.stringify(join(workspaceRoot, 'fixtures/fonts/ttf/roboto-regular.ttf'))})
    const info = inspect(input)

    if (info.format !== 'ttf') throw new Error('native binding did not inspect the TTF fixture')
  `

  await runConsumer(
    [bindingTarball, platformTarball, wasmTarball, nodeTarball],
    nativeInspectSource,
  )
  await runConsumer(
    [wasmTarball],
    "import { initWasm } from '@fontmin-rs/wasm'; if (typeof initWasm !== 'function') throw new Error('missing WASM init export')",
  )
  const autoOptimizeSource = `
    import { readFile } from 'node:fs/promises'
    import { modernWeb, optimize } from 'fontmin-rs'

    const assets = await optimize({
      input: [await readFile(${JSON.stringify(join(workspaceRoot, 'fixtures/fonts/ttf/roboto-regular.ttf'))})],
      runtime: 'auto',
      plugins: modernWeb({ text: 'Hello' }),
    })
    const hasWoff2 = assets.some(asset =>
      Buffer.from(asset.contents).subarray(0, 4).toString('ascii') === 'wOF2',
    )

    if (!hasWoff2) throw new Error('auto optimize did not use WASM without a native artifact')
  `

  await runConsumer([wasmTarball, nodeTarball], autoOptimizeSource)
} finally {
  await rm(tarballRoot, { force: true, recursive: true })
}
