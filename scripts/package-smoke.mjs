import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { copyFile, mkdtemp, readdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
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

async function runConsumer(tarballs, source, fixtures, beforeRun) {
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
    await Promise.all(
      (fixtures ?? []).map(({ destination, source }) =>
        copyFile(source, join(directory, destination)),
      ),
    )
    await executeFile('npm', ['install', '--ignore-scripts', ...tarballs], {
      cwd: directory,
    })
    if (beforeRun !== undefined) {
      await beforeRun(directory)
    }
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

export async function removeNativeArtifacts(nodeModules) {
  const scopeDir = join(nodeModules, '@fontmin-rs')
  let entries = []

  try {
    entries = await readdir(scopeDir, { withFileTypes: true })
  } catch (error) {
    if (error?.code === 'ENOENT') {
      return
    }
    throw error
  }

  await Promise.all(
    entries
      .filter(entry => entry.isDirectory() && entry.name.startsWith('binding-'))
      .map(entry =>
        rm(join(scopeDir, entry.name), { force: true, recursive: true }),
      ),
  )
  await removeNodeFiles(join(scopeDir, 'binding'))
}

export async function prepareAutoFallbackConsumer(directory) {
  await removeNativeArtifacts(join(directory, 'node_modules'))
}

async function removeNodeFiles(directory) {
  let entries = []

  try {
    entries = await readdir(directory, { withFileTypes: true })
  } catch (error) {
    if (error?.code === 'ENOENT') {
      return
    }
    throw error
  }

  await Promise.all(
    entries.map(async entry => {
      const path = join(directory, entry.name)

      if (entry.isDirectory()) {
        await removeNodeFiles(path)
      } else if (entry.isFile() && entry.name.endsWith('.node')) {
        await rm(path)
      }
    }),
  )
}

export async function packageSmoke() {
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
    await runConsumer(
      [wasmTarball, nodeTarball],
      `import { inspect, modernWeb, optimize } from 'fontmin-rs'
let nativeUnavailable = false
try {
  inspect(new Uint8Array())
} catch (error) {
  if (error?.name !== 'NativeBindingLoadError') throw error
  nativeUnavailable = true
}
if (!nativeUnavailable) throw new Error('native API unexpectedly loaded without a binding')
const assets = await optimize({
  input: ['./roboto.ttf'],
  runtime: 'auto',
  plugins: modernWeb({ text: 'Hello' }),
})
if (!assets.some(asset => Buffer.from(asset.contents).subarray(0, 4).toString('ascii') === 'wOF2')) {
  throw new Error('auto optimize did not use WASM without a native artifact')
}`,
      [
        {
          destination: 'roboto.ttf',
          source: join(workspaceRoot, 'fixtures/fonts/ttf/roboto-regular.ttf'),
        },
      ],
      prepareAutoFallbackConsumer,
    )
  } finally {
    await rm(tarballRoot, { force: true, recursive: true })
  }
}

if (
  process.argv[1] !== undefined &&
  pathToFileURL(resolve(process.argv[1])).href === import.meta.url
) {
  await packageSmoke()
}
