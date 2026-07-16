import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { promisify } from 'node:util'
import { checkReleaseReadiness } from './release-readiness.mjs'

const executeFile = promisify(execFile)
const repository = {
  type: 'git',
  url: 'git+https://github.com/fontmin-rs/fontmin-rs.git',
}
const platformPackages = [
  ['binding-darwin-arm64', '@fontmin-rs/binding-darwin-arm64'],
  ['binding-darwin-x64', '@fontmin-rs/binding-darwin-x64'],
  ['binding-linux-arm64-gnu', '@fontmin-rs/binding-linux-arm64-gnu'],
  ['binding-linux-arm64-musl', '@fontmin-rs/binding-linux-arm64-musl'],
  ['binding-linux-x64-gnu', '@fontmin-rs/binding-linux-x64-gnu'],
  ['binding-linux-x64-musl', '@fontmin-rs/binding-linux-x64-musl'],
  ['binding-win32-arm64-msvc', '@fontmin-rs/binding-win32-arm64-msvc'],
  ['binding-win32-x64-msvc', '@fontmin-rs/binding-win32-x64-msvc'],
]

async function createReleaseWorkspace({ changelogVersion, versions }) {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-release-readiness-'))

  await writeFile(
    join(root, 'package.json'),
    JSON.stringify({
      name: 'fontmin-rs-monorepo',
      private: true,
      version: versions.root,
    }),
  )
  await writeFile(
    join(root, 'Cargo.toml'),
    `[workspace.package]\nversion = "${versions.rust}"\n`,
  )
  await writeFile(
    join(root, 'CHANGELOG.md'),
    `# Changelog\n\n## [${changelogVersion}] - 2026-07-13\n`,
  )

  for (const [directory, name, version] of [
    ['packages/fontmin', 'fontmin-rs', versions.node],
    ['napi/fontmin', '@fontmin-rs/binding', versions.binding],
    ['wasm/fontmin', '@fontmin-rs/wasm', versions.wasm],
    ...platformPackages.map(([directory, name]) => [
      `npm/${directory}`,
      name,
      versions.platform,
    ]),
  ]) {
    const packageDirectory = join(root, directory)
    await mkdir(packageDirectory, { recursive: true })
    await writeFile(
      join(packageDirectory, 'package.json'),
      JSON.stringify({
        license: 'MIT',
        name,
        publishConfig: { access: 'public' },
        repository,
        version,
      }),
    )
  }

  for (const [path, contents] of [
    [
      'packages/fontmin/src/optimize.ts',
      `const FONTMIN_VERSION = '${versions.source}'`,
    ],
    [
      'packages/fontmin/bin/fontmin-rs.mjs',
      `const FONTMIN_VERSION = '${versions.bin}'`,
    ],
    [
      'napi/fontmin/src-js/bindings.js',
      `if (bindingPackageVersion !== '${versions.generatedBinding}') throw new Error('Native binding package version mismatch, expected ${versions.generatedBinding} but got another version')`,
    ],
  ]) {
    const filePath = join(root, path)
    await mkdir(join(filePath, '..'), { recursive: true })
    await writeFile(filePath, contents)
  }

  return root
}

const releaseVersions = {
  binding: '0.1.0-beta.1',
  bin: '0.1.0-beta.1',
  generatedBinding: '0.1.0-beta.1',
  node: '0.1.0-beta.1',
  platform: '0.1.0-beta.1',
  root: '0.1.0-beta.1',
  rust: '0.1.0-beta.1',
  source: '0.1.0-beta.1',
  wasm: '0.1.0-beta.1',
}

test('accepts one release version across every published package', async () => {
  const root = await createReleaseWorkspace({
    changelogVersion: '0.1.0-beta.1',
    versions: releaseVersions,
  })

  try {
    const result = await checkReleaseReadiness({
      root,
      tag: 'v0.1.0-beta.1',
    })

    assert.equal(result.version, '0.1.0-beta.1')
    assert.equal(result.packages.length, 11)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects a missing platform package', async () => {
  const root = await createReleaseWorkspace({
    changelogVersion: '0.1.0-beta.1',
    versions: releaseVersions,
  })

  try {
    await rm(join(root, 'npm/binding-linux-x64-musl'), {
      recursive: true,
    })
    await assert.rejects(
      checkReleaseReadiness({ root }),
      /missing published package directory npm\/binding-linux-x64-musl/u,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects an unexpected publishable platform package', async () => {
  const root = await createReleaseWorkspace({
    changelogVersion: '0.1.0-beta.1',
    versions: releaseVersions,
  })

  try {
    const packageRoot = join(root, 'npm/binding-unexpected')
    await mkdir(packageRoot, { recursive: true })
    await writeFile(
      join(packageRoot, 'package.json'),
      JSON.stringify({
        name: '@fontmin-rs/binding-unexpected',
        publishConfig: { access: 'public' },
        version: '0.1.0-beta.1',
      }),
    )
    await assert.rejects(
      checkReleaseReadiness({ root }),
      /unexpected published package directory npm\/binding-unexpected/u,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects mixed embedded versions', async () => {
  const root = await createReleaseWorkspace({
    changelogVersion: '0.1.0-beta.1',
    versions: releaseVersions,
  })

  try {
    await writeFile(
      join(root, 'napi/fontmin/src-js/bindings.js'),
      "if (bindingPackageVersion !== '0.0.0') throw new Error('Native binding package version mismatch, expected 0.1.0-beta.1 but got another')\n",
    )
    await assert.rejects(
      checkReleaseReadiness({ root }),
      /bindings\.js must only embed 0\.1\.0-beta\.1/u,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects a root package version mismatch', async () => {
  const root = await createReleaseWorkspace({
    changelogVersion: '0.1.0-beta.1',
    versions: { ...releaseVersions, root: '0.1.0-beta.2' },
  })

  try {
    await assert.rejects(
      checkReleaseReadiness({ root }),
      /root package\.json has version 0\.1\.0-beta\.2/u,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects incomplete package publishing metadata', async () => {
  const root = await createReleaseWorkspace({
    changelogVersion: '0.1.0-beta.1',
    versions: releaseVersions,
  })

  try {
    const manifestPath = join(root, 'wasm/fontmin/package.json')
    const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))
    delete manifest.license
    manifest.repository.url = 'https://example.com/wrong.git'
    await writeFile(manifestPath, JSON.stringify(manifest))

    await assert.rejects(checkReleaseReadiness({ root }), error => {
      assert.match(error.message, /@fontmin-rs\/wasm must set license to MIT/u)
      assert.match(error.message, /@fontmin-rs\/wasm must set repository\.url/u)
      return true
    })
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects a tag flag without a value', async () => {
  await assert.rejects(
    executeFile(process.execPath, ['scripts/release-readiness.mjs', '--tag']),
    error => {
      assert.match(error.stderr, /--tag requires a value/u)
      return true
    },
  )
})

test('reports every version and changelog mismatch before release', async () => {
  const root = await createReleaseWorkspace({
    changelogVersion: '0.1.0-beta.2',
    versions: {
      ...releaseVersions,
      bin: '0.0.0',
      platform: '0.0.0',
      wasm: '0.1.0-beta.2',
    },
  })

  try {
    await assert.rejects(
      checkReleaseReadiness({ root, tag: 'v0.1.0-beta.3' }),
      error => {
        assert.match(error.message, /@fontmin-rs\/wasm.*0\.1\.0-beta\.2/u)
        assert.match(error.message, /binding-darwin-arm64.*0\.0\.0/u)
        assert.match(error.message, /fontmin-rs\.mjs.*0\.1\.0-beta\.1/u)
        assert.match(error.message, /CHANGELOG\.md.*0\.1\.0-beta\.1/u)
        assert.match(error.message, /tag.*v0\.1\.0-beta\.3/u)
        return true
      },
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})
