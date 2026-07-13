import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { mkdtemp, readdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(dirname(fileURLToPath(import.meta.url)))

async function packPackage(directory, tarballDirectory) {
  await executeFile('pnpm', ['pack', '--pack-destination', tarballDirectory], {
    cwd: join(workspaceRoot, directory),
  })

  const tarballs = (await readdir(tarballDirectory))
    .filter(fileName => fileName.endsWith('.tgz'))
    .map(fileName => join(tarballDirectory, fileName))

  assert.equal(tarballs.length, 1, `expected one tarball for ${directory}`)
  return tarballs[0]
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
  const nodeTarball = await packPackage(
    'packages/fontmin',
    join(tarballRoot, 'node'),
  )
  const wasmTarball = await packPackage(
    'wasm/fontmin',
    join(tarballRoot, 'wasm'),
  )

  await runConsumer(
    [bindingTarball, wasmTarball, nodeTarball],
    "import { inspect, ttfToWoff2Async } from 'fontmin-rs'; if (typeof inspect !== 'function' || typeof ttfToWoff2Async !== 'function') throw new Error('missing Node fallback export')",
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
