import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { copyFile, mkdtemp, readdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)

export function currentPlatformPackageDirectory() {
  const architectures = new Set(['arm64', 'x64'])
  assert.ok(
    architectures.has(process.arch),
    `unsupported package-smoke architecture ${process.arch}`,
  )

  if (process.platform === 'darwin') {
    return `npm/binding-darwin-${process.arch}`
  }
  if (process.platform === 'win32') {
    return `npm/binding-win32-${process.arch}-msvc`
  }
  if (process.platform === 'linux') {
    const report = process.report?.getReport()
    const libc = report?.header?.glibcVersionRuntime ? 'gnu' : 'musl'
    return `npm/binding-linux-${process.arch}-${libc}`
  }

  throw new Error(`unsupported package-smoke platform ${process.platform}`)
}

export async function packPackage(directory, tarballDirectory) {
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
    const platformTarball = await packPackage(
      currentPlatformPackageDirectory(),
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

    await runConsumer(
      [platformTarball, bindingTarball, wasmTarball, nodeTarball],
      `const [
  main,
  plugins,
  presets,
  compat,
] = await Promise.all([
  import('fontmin-rs'),
  import('fontmin-rs/plugins'),
  import('fontmin-rs/presets'),
  import('fontmin-rs/compat'),
])
if (typeof main.inspect !== 'function' || typeof main.ttfToWoff2Async !== 'function') {
  throw new Error('missing Node entry-point export')
}
if (typeof plugins.glyph !== 'function' || typeof presets.modernWeb !== 'function') {
  throw new Error('missing Node subpath export')
}
if (typeof compat.default !== 'function') throw new Error('missing compat default export')`,
    )
    await runConsumer(
      [wasmTarball],
      "import { initWasm } from '@fontmin-rs/wasm'; if (typeof initWasm !== 'function') throw new Error('missing WASM init export')",
    )
    await runConsumer(
      [platformTarball, bindingTarball, wasmTarball, nodeTarball],
      `import { readFile } from 'node:fs/promises'
import { inspect, modernWeb, optimize } from 'fontmin-rs'
const input = await readFile('./roboto.ttf')
const info = inspect(input)
if (info.format !== 'ttf' || info.metadata.familyName !== 'Roboto') {
  throw new Error(\`native inspect returned unexpected metadata: \${JSON.stringify(info)}\`)
}
const assets = await optimize({
  input: ['./roboto.ttf'],
  runtime: 'native',
  plugins: modernWeb({ text: 'Hello' }),
})
const names = assets.map(asset => asset.path).sort()
for (const expected of ['roboto.css', 'roboto.ttf', 'roboto.woff', 'roboto.woff2']) {
  if (!names.some(name => name.endsWith(expected))) {
    throw new Error(\`native tarball pipeline omitted \${expected}: \${names.join(', ')}\`)
  }
}`,
      [
        {
          destination: 'roboto.ttf',
          source: join(workspaceRoot, 'fixtures/fonts/ttf/roboto-regular.ttf'),
        },
      ],
    )
    await runConsumer(
      [platformTarball, bindingTarball, wasmTarball, nodeTarball],
      `import { execFileSync } from 'node:child_process'
import { resolve } from 'node:path'
const bin = resolve('node_modules/fontmin-rs/bin/fontmin-rs.mjs')
const stdout = execFileSync(process.execPath, [bin, 'inspect', 'roboto.ttf', '--json'], {
  encoding: 'utf8',
})
const info = JSON.parse(stdout)
if (info.format !== 'ttf' || info.metadata.familyName !== 'Roboto') {
  throw new Error(\`packed CLI returned unexpected metadata: \${stdout}\`)
}`,
      [
        {
          destination: 'roboto.ttf',
          source: join(workspaceRoot, 'fixtures/fonts/ttf/roboto-regular.ttf'),
        },
      ],
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
    await runConsumer(
      [wasmTarball, nodeTarball],
      `import { modernWeb, optimize } from 'fontmin-rs'
const assets = await optimize({
  input: ['./roboto.ttf'],
  runtime: 'wasm',
  plugins: modernWeb({ text: 'Hello' }),
})
if (!assets.some(asset => Buffer.from(asset.contents).subarray(0, 4).toString('ascii') === 'wOF2')) {
  throw new Error('forced-WASM optimize did not emit WOFF2')
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
