import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { copyFile, mkdtemp, readdir, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'
import {
  findNativeReleaseEntry,
  readNativeReleaseLayout,
} from './native-release-layout.mjs'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)

export async function currentPlatformPackageDirectory() {
  let libc

  if (process.platform === 'linux') {
    const report = process.report?.getReport()
    libc = report?.header?.glibcVersionRuntime ? 'glibc' : 'musl'
  }

  const { entries } = await readNativeReleaseLayout()
  return findNativeReleaseEntry(entries, {
    arch: process.arch,
    libc,
    platform: process.platform,
  }).directory
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

export function registryInstallSpecs(version) {
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(version)) {
    throw new Error(
      `registry compatibility checks require exact SemVer: ${version}`,
    )
  }

  return {
    full: [`fontmin-rs@${version}`],
    wasm: [`@fontmin-rs/wasm@${version}`],
  }
}

async function runConsumer(installSpecs, source, fixtures, beforeRun) {
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
    await executeFile('npm', ['install', '--ignore-scripts', ...installSpecs], {
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

export async function packageSmoke({
  registryVersion = process.env.FONTMIN_REGISTRY_VERSION,
} = {}) {
  const tarballRoot = await mkdtemp(join(tmpdir(), 'fontmin-tarballs-'))

  try {
    let fullInstallSpecs
    let isolatedInstallSpecs
    let wasmInstallSpecs

    if (registryVersion === undefined) {
      const bindingTarball = await packPackage(
        'napi/fontmin',
        join(tarballRoot, 'binding'),
      )
      const platformTarball = await packPackage(
        await currentPlatformPackageDirectory(),
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

      fullInstallSpecs = [
        platformTarball,
        bindingTarball,
        wasmTarball,
        nodeTarball,
      ]
      isolatedInstallSpecs = [wasmTarball, nodeTarball]
      wasmInstallSpecs = [wasmTarball]
    } else {
      const registrySpecs = registryInstallSpecs(registryVersion)

      fullInstallSpecs = registrySpecs.full
      isolatedInstallSpecs = registrySpecs.full
      wasmInstallSpecs = registrySpecs.wasm
    }

    await runConsumer(
      fullInstallSpecs,
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
      wasmInstallSpecs,
      "import { initWasm } from '@fontmin-rs/wasm'; if (typeof initWasm !== 'function') throw new Error('missing WASM init export')",
    )
    await runConsumer(
      fullInstallSpecs,
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
      fullInstallSpecs,
      `import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
const bin = resolve('node_modules/fontmin-rs/bin/fontmin-rs.mjs')
const help = execFileSync(process.execPath, [bin, '--help'], { encoding: 'utf8' })
for (const flag of ['--css-unicode-range', '--delivery-slice']) {
  if (!help.includes(flag)) throw new Error(\`packed CLI help omitted \${flag}\`)
}
const stdout = execFileSync(process.execPath, [bin, 'inspect', 'roboto.ttf', '--json'], {
  encoding: 'utf8',
})
const info = JSON.parse(stdout)
if (info.format !== 'ttf' || info.metadata.familyName !== 'Roboto') {
  throw new Error(\`packed CLI returned unexpected metadata: \${stdout}\`)
}
execFileSync(process.execPath, [
  bin,
  'build',
  'roboto.ttf',
  '--out-dir',
  'css-dist',
  '--formats',
  'woff2,css',
  '--css-unicode-range',
  'u+20-7e',
])
const css = readFileSync('css-dist/roboto.css', 'utf8')
if (!css.includes('unicode-range: U+0020-007E;')) {
  throw new Error(\`packed CLI omitted its CSS Unicode range: \${css}\`)
}
execFileSync(process.execPath, [
  bin,
  'build',
  'roboto.ttf',
  '--out-dir',
  'delivery-dist',
  '--formats',
  'woff2,css',
  '--delivery-slice',
  'latin:U+0041-005A',
])
const deliveryCss = readFileSync('delivery-dist/roboto-latin.css', 'utf8')
if (!deliveryCss.includes('unicode-range: U+0041-005A;')) {
  throw new Error(\`packed CLI omitted its delivery slice: \${deliveryCss}\`)
}`,
      [
        {
          destination: 'roboto.ttf',
          source: join(workspaceRoot, 'fixtures/fonts/ttf/roboto-regular.ttf'),
        },
      ],
    )
    await runConsumer(
      isolatedInstallSpecs,
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
      isolatedInstallSpecs,
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
