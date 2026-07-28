import { execSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'

interface PackageManifest {
  bugs?: {
    url?: string
  }
  devDependencies?: Record<string, string>
  engines?: {
    node?: string
  }
  exports?: Record<
    string,
    | string
    | {
        default?: string
        types?: string
      }
  >
  optionalDependencies?: Record<string, string>
  homepage?: string
  private?: boolean
  publishConfig?: {
    access?: string
  }
  repository?: {
    type?: string
    url?: string
  }
  scripts?: Record<string, string>
}

interface PackFile {
  path: string
}

interface PackManifest {
  files: PackFile[]
  name: string
  version: string
}

interface PlatformPackage {
  cpu: string[]
  dir: string
  libc?: string[]
  name: string
  nodeFile: string
  os: string[]
}

const packageRoot = resolve(import.meta.dirname, '..')
const manifest = JSON.parse(
  readFileSync(resolve(packageRoot, 'package.json'), 'utf8'),
) as PackageManifest
const bindingManifest = JSON.parse(
  readFileSync(resolve(packageRoot, '../../napi/fontmin/package.json'), 'utf8'),
) as PackageManifest
const rootManifest = JSON.parse(
  readFileSync(resolve(packageRoot, '../../package.json'), 'utf8'),
) as PackageManifest
const repositoryRoot = resolve(packageRoot, '../..')
const platformPackages: PlatformPackage[] = [
  {
    cpu: ['arm64'],
    dir: 'npm/binding-darwin-arm64',
    name: '@fontmin-rs/binding-darwin-arm64',
    nodeFile: 'fontmin_rs.darwin-arm64.node',
    os: ['darwin'],
  },
  {
    cpu: ['x64'],
    dir: 'npm/binding-darwin-x64',
    name: '@fontmin-rs/binding-darwin-x64',
    nodeFile: 'fontmin_rs.darwin-x64.node',
    os: ['darwin'],
  },
  {
    cpu: ['arm64'],
    dir: 'npm/binding-linux-arm64-gnu',
    libc: ['glibc'],
    name: '@fontmin-rs/binding-linux-arm64-gnu',
    nodeFile: 'fontmin_rs.linux-arm64-gnu.node',
    os: ['linux'],
  },
  {
    cpu: ['arm64'],
    dir: 'npm/binding-linux-arm64-musl',
    libc: ['musl'],
    name: '@fontmin-rs/binding-linux-arm64-musl',
    nodeFile: 'fontmin_rs.linux-arm64-musl.node',
    os: ['linux'],
  },
  {
    cpu: ['x64'],
    dir: 'npm/binding-linux-x64-gnu',
    libc: ['glibc'],
    name: '@fontmin-rs/binding-linux-x64-gnu',
    nodeFile: 'fontmin_rs.linux-x64-gnu.node',
    os: ['linux'],
  },
  {
    cpu: ['x64'],
    dir: 'npm/binding-linux-x64-musl',
    libc: ['musl'],
    name: '@fontmin-rs/binding-linux-x64-musl',
    nodeFile: 'fontmin_rs.linux-x64-musl.node',
    os: ['linux'],
  },
  {
    cpu: ['arm64'],
    dir: 'npm/binding-win32-arm64-msvc',
    name: '@fontmin-rs/binding-win32-arm64-msvc',
    nodeFile: 'fontmin_rs.win32-arm64-msvc.node',
    os: ['win32'],
  },
  {
    cpu: ['x64'],
    dir: 'npm/binding-win32-x64-msvc',
    name: '@fontmin-rs/binding-win32-x64-msvc',
    nodeFile: 'fontmin_rs.win32-x64-msvc.node',
    os: ['win32'],
  },
]

it('exposes benchmark scripts and benchmark files', () => {
  expect(rootManifest.scripts?.['bench']).toBe(
    'cargo bench --workspace && pnpm --filter fontmin-rs bench',
  )
  expect(manifest.scripts?.['bench']).toBe('vitest bench --run bench')
  expect(manifest.devDependencies?.['fontmin']).toBeDefined()
  expect(manifest.devDependencies?.['tinybench']).toBeDefined()
  expect(existsSync(resolve(packageRoot, 'bench/fontmin.bench.ts'))).toBe(true)
  expect(existsSync(resolve(packageRoot, 'bench/subset.bench.ts'))).toBe(true)
  expect(existsSync(resolve(packageRoot, 'bench/convert.bench.ts'))).toBe(true)
})

it('exposes browser load test tooling', () => {
  expect(manifest.scripts?.['test:browser']).toBe(
    'pnpm run build && node tests/browser-load.mjs',
  )
  expect(manifest.devDependencies?.['playwright']).toBeDefined()
  expect(existsSync(resolve(packageRoot, 'tests/browser-load.mjs'))).toBe(true)
})

it('declares package export entries', () => {
  expect(manifest.exports?.['./plugins']).toStrictEqual({
    default: './dist/plugins.mjs',
    types: './dist/plugins.d.mts',
  })
  expect(manifest.exports?.['./presets']).toStrictEqual({
    default: './dist/presets.mjs',
    types: './dist/presets.d.mts',
  })
  expect(manifest.exports?.['./compat']).toStrictEqual({
    default: './dist/compat.mjs',
    types: './dist/compat.d.mts',
  })
})

it('declares the tested Node.js version floor', () => {
  expect(manifest.engines?.node).toBe('>=22.18.0')
})

it('keeps the packaged executable as a thin adapter', () => {
  const executable = readFileSync(
    resolve(packageRoot, 'bin/fontmin-rs.mjs'),
    'utf8',
  )
  const executableLines = executable.trim().split('\n')

  expect(executableLines.length).toBeLessThanOrEqual(5)
  expect(executable).toContain("from '../dist/cli.mjs'")
  expect(executable).not.toContain('@fontmin-rs/binding')
})

it('routes CLI builds through the shared optimizer', () => {
  const cliRuntime = readFileSync(resolve(packageRoot, 'src/cli.mjs'), 'utf8')

  expect(cliRuntime).toContain("from './optimize'")
  expect(cliRuntime).not.toContain('svgsToTtf')
  expect(cliRuntime).not.toContain('cacheKeyForBuildInput')
  expect(cliRuntime).not.toContain('generateFontFaceCss')
})

it('separates optimizer responsibilities behind a thin facade', () => {
  const facade = readFileSync(resolve(packageRoot, 'src/optimize.ts'), 'utf8')
  const pipeline = readFileSync(
    resolve(packageRoot, 'src/optimize-pipeline.ts'),
    'utf8',
  )
  const storage = readFileSync(
    resolve(packageRoot, 'src/optimize-storage.ts'),
    'utf8',
  )
  const transforms = readFileSync(
    resolve(packageRoot, 'src/optimize-transforms.ts'),
    'utf8',
  )

  expect(facade.trim().split('\n').length).toBeLessThanOrEqual(3)
  expect(pipeline).toContain("from './optimize-storage'")
  expect(pipeline).toContain("from './optimize-transforms'")
  expect(storage).toContain("from 'node:fs/promises'")
  expect(storage).toContain('CACHE_SCHEMA_VERSION')
  expect(transforms).not.toContain("from 'node:fs/promises'")
})

it('organizes integration tests by public API and CLI command seams', () => {
  const nodeTestFiles = [
    'api-font.test.ts',
    'cli-core.test.ts',
    'cli-build.test.ts',
    'optimize-core.test.ts',
    'config-and-plugins.test.ts',
    'presets-and-cache.test.ts',
  ]
  const rustTestFiles = [
    'contract.rs',
    'subset.rs',
    'convert.rs',
    'build.rs',
    'maintenance.rs',
    'config.rs',
  ]

  expect(existsSync(resolve(packageRoot, 'tests/api.test.ts'))).toBe(false)
  for (const file of nodeTestFiles) {
    const source = readFileSync(resolve(packageRoot, 'tests', file), 'utf8')

    expect(source.trim().split('\n').length).toBeLessThan(2_000)
  }

  const rustTestRoot = resolve(repositoryRoot, 'apps/fontmin/tests')
  const rustTestEntry = readFileSync(resolve(rustTestRoot, 'cli.rs'), 'utf8')

  expect(rustTestEntry.trim().split('\n').length).toBeLessThan(100)
  for (const file of rustTestFiles) {
    expect(rustTestEntry).toContain(`mod ${file.replace('.rs', '')};`)
    const source = readFileSync(resolve(rustTestRoot, 'cli', file), 'utf8')

    expect(source.trim().split('\n').length).toBeLessThan(2_000)
  }
})

it('packs the published package entry points', () => {
  const packed = JSON.parse(
    execSync('pnpm pack --dry-run --json', {
      cwd: packageRoot,
      encoding: 'utf8',
    }),
  ) as PackManifest
  const files = packed.files.map(file => file.path)

  expect(packed.name).toBe('fontmin-rs')
  expect(files).toContain('bin/fontmin-rs.mjs')
  expect(files).toContain('dist/cli.mjs')
  expect(files).toContain('dist/index.mjs')
  expect(files).toContain('dist/index.d.mts')
  expect(files).toContain('dist/plugins.mjs')
  expect(files).toContain('dist/plugins.d.mts')
  expect(files).toContain('dist/presets.mjs')
  expect(files).toContain('dist/presets.d.mts')
  expect(files).toContain('dist/compat.mjs')
  expect(files).toContain('dist/compat.d.mts')
  expect(files).toContain('LICENSE')
  expect(files).toContain('package.json')
  expect(files.some(file => file.startsWith('src/'))).toBe(false)
  expect(files.some(file => file.startsWith('tests/'))).toBe(false)
})

it('defines repository ci gates', () => {
  const workflowPath = resolve(repositoryRoot, '.github/workflows/ci.yml')

  expect(existsSync(workflowPath)).toBe(true)

  const workflow = readFileSync(workflowPath, 'utf8')
  const checkJob = workflow.slice(
    workflow.indexOf('  check:'),
    workflow.indexOf('  release-readiness:'),
  )
  const checkSteps = checkJob.split('\n').map(line => line.trim())
  const testJob = workflow.slice(
    workflow.indexOf('  test:'),
    workflow.indexOf('  bench:'),
  )

  expect(workflow).toContain('pnpm run format:check')
  expect(workflow).toContain('pnpm run lint')
  expect(workflow).toContain('targets: wasm32-unknown-unknown')
  expect(workflow).toContain('tool: wasm-pack@0.15.0')
  expect(workflow).not.toContain('jetli/wasm-pack-action')
  expect(checkSteps).toContain('- run: node scripts/ensure-wasm.mjs')
  expect(
    checkSteps.indexOf('- run: node scripts/ensure-wasm.mjs'),
  ).toBeLessThan(checkSteps.indexOf('- run: pnpm run typecheck'))
  expect(testJob).toContain("FONTMIN_WASM_PREBUILT: '1'")
  expect(workflow).toContain('pnpm run typecheck')
  expect(workflow).toContain('  msrv:')
  expect(workflow).toContain(
    'dtolnay/rust-toolchain@39b0b3842c7e8bbf6904c0bfc3d9006fdd4dc4e0 # 1.88.0',
  )
  expect(workflow).toContain(
    'cargo check --locked --workspace --all-targets --all-features',
  )
  expect(workflow).toContain('needs: [check, msrv]')
  expect(workflow).toContain('pnpm run test')
  expect(workflow).toContain('pnpm run build')
  expect(workflow).toContain('pnpm run bench:report')
  expect(workflow).toContain('benchmarks/current.json')
  expect(workflow).toContain('benchmarks/production-current.json')
  expect(workflow).toContain('pnpm run bench:production')
  expect(workflow).toContain('runs-on: ubuntu-24.04')
  expect(workflow).toContain('node-version: 24.x')
  expect(rootManifest.scripts?.['bench:report']).toContain(
    'pnpm run build:release',
  )
  expect(rootManifest.scripts?.['bench:report']).toContain('--profile release')
  expect(rootManifest.scripts?.['bench:report']).toContain('--trials 3')
  expect(workflow).toContain('browser-load:')
  expect(workflow).toContain(
    'pnpm --filter fontmin-rs exec playwright install --with-deps chromium',
  )
  expect(workflow).toContain('pnpm --filter fontmin-rs test:browser')
  expect(workflow).toContain('name: wasm-bindings')
  expect(testJob).toContain('actions/download-artifact')
  expect(testJob).toContain('path: wasm/fontmin/src/generated')
})

it('keeps repository automation and metadata aligned with the canonical URL', () => {
  const gitignore = readFileSync(resolve(repositoryRoot, '.gitignore'), 'utf8')
  const cargoManifest = readFileSync(
    resolve(repositoryRoot, 'Cargo.toml'),
    'utf8',
  )

  expect(
    existsSync(resolve(repositoryRoot, '.github/workflows/autofix.yml')),
  ).toBe(false)
  expect(gitignore).toContain('.worktrees')
  expect(cargoManifest).toContain(
    'repository = "https://github.com/fontmin-rs/fontmin-rs"',
  )
  expect(manifest.homepage).toBe(
    'https://github.com/fontmin-rs/fontmin-rs#readme',
  )
  expect(manifest.bugs?.url).toBe(
    'https://github.com/fontmin-rs/fontmin-rs/issues',
  )
  expect(manifest.repository?.url).toBe(
    'git+https://github.com/fontmin-rs/fontmin-rs.git',
  )
  expect(manifest.scripts?.['pretest']).toBe(
    'pnpm run build && node ../../scripts/ensure-wasm.mjs',
  )
})

it('declares native platform packages for publish artifacts', () => {
  const optionalDependencies = Object.fromEntries(
    platformPackages.map(platformPackage => [
      platformPackage.name,
      'workspace:*',
    ]),
  )

  expect(bindingManifest.optionalDependencies).toMatchObject(
    optionalDependencies,
  )

  for (const platformPackage of platformPackages) {
    const manifestPath = resolve(
      repositoryRoot,
      platformPackage.dir,
      'package.json',
    )

    expect(existsSync(manifestPath)).toBe(true)

    const platformManifest = JSON.parse(
      readFileSync(manifestPath, 'utf8'),
    ) as PackageManifest & {
      cpu?: string[]
      files?: string[]
      libc?: string[]
      main?: string
      name?: string
      os?: string[]
      private?: boolean
    }

    expect(platformManifest.name).toBe(platformPackage.name)
    expect(platformManifest.private).not.toBe(true)
    expect(platformManifest.main).toBe(platformPackage.nodeFile)
    expect(platformManifest.files).toContain(platformPackage.nodeFile)
    expect(platformManifest.os).toStrictEqual(platformPackage.os)
    expect(platformManifest.cpu).toStrictEqual(platformPackage.cpu)
    expect(platformManifest.libc).toStrictEqual(platformPackage.libc)
  }
})

it('wires native release artifact scripts and ci job', () => {
  const workflow = readFileSync(
    resolve(repositoryRoot, '.github/workflows/ci.yml'),
    'utf8',
  )

  expect(bindingManifest.private).not.toBe(true)
  expect(bindingManifest.publishConfig?.access).toBe('public')
  expect(bindingManifest.scripts?.['artifacts']).toBe(
    'node ../../scripts/copy-native-artifacts.mjs',
  )
  expect(bindingManifest.scripts?.['build:npm-dir']).toBe(
    'napi create-npm-dirs --npm-dir ../../npm && pnpm run artifacts',
  )
  expect(bindingManifest.scripts?.['prepublishOnly']).toBe(
    'napi pre-publish -t npm --no-gh-release',
  )

  expect(workflow).toContain('build-native:')
  expect(workflow).toContain('target:')
  expect(workflow).toContain('x86_64-apple-darwin')
  expect(workflow).toContain('aarch64-apple-darwin')
  expect(workflow).toContain('x86_64-pc-windows-msvc')
  expect(workflow).toContain('aarch64-pc-windows-msvc')
  expect(workflow).toContain('x86_64-unknown-linux-gnu')
  expect(workflow).toContain('x86_64-unknown-linux-musl')
  expect(workflow).toContain('aarch64-unknown-linux-gnu')
  expect(workflow).toContain('aarch64-unknown-linux-musl')
  expect(workflow).toContain('build_args: --use-napi-cross')
  expect(workflow).toContain('build_args: --cross-compile')
  expect(workflow).toContain(
    'mlugg/setup-zig@d1434d08867e3ee9daa34448df10607b98908d29 # v2',
  )
  expect(workflow).toContain('tool: cargo-zigbuild')
  expect(workflow).toContain(
    'pnpm --filter @fontmin-rs/binding build --target ${{ matrix.target }} ${{ matrix.build_args }}',
  )
  expect(workflow).toContain('pnpm --filter @fontmin-rs/binding artifacts')
  expect(workflow).toContain('actions/upload-artifact')
})

it('wires release publishing through native artifacts', () => {
  const workflow = readFileSync(
    resolve(repositoryRoot, '.github/workflows/release.yml'),
    'utf8',
  )

  expect(workflow).toContain('build-native:')
  expect(workflow).toContain('publish:')
  expect(workflow).toContain('needs: [build-native]')
  expect(workflow).toContain('build_args: --use-napi-cross')
  expect(workflow).toContain('build_args: --cross-compile')
  expect(workflow).toContain(
    'mlugg/setup-zig@d1434d08867e3ee9daa34448df10607b98908d29',
  )
  expect(workflow).toContain('tool: cargo-zigbuild')
  expect(workflow).toContain(
    'pnpm --filter @fontmin-rs/binding build --target ${{ matrix.target }} ${{ matrix.build_args }}',
  )
  expect(workflow).toContain('pnpm --filter @fontmin-rs/binding artifacts')
  expect(workflow).toContain('actions/upload-artifact')
  expect(workflow).toContain('actions/download-artifact')
  expect(workflow).toContain('Place downloaded native artifacts')
  expect(workflow).toContain('Verify native package artifacts')
  expect(workflow).toContain('Missing native artifacts:')
  expect(workflow).toContain('pnpm --filter fontmin-rs build')
  expect(workflow).toContain(
    'pnpm -r publish --no-git-checks --access public --report-summary',
  )
  expect(workflow).toContain('id-token: write')
  expect(workflow).toContain('--provenance')
  expect(workflow).not.toContain('NODE_AUTH_TOKEN')
  expect(workflow).not.toContain('NPM_TOKEN')
})
