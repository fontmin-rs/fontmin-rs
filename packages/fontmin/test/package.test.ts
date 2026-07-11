import { execFileSync } from 'node:child_process'
import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import { parse } from 'yaml'

interface PackageManifest {
  bugs?: {
    url?: string
  }
  devDependencies?: Record<string, string>
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

interface WorkflowConfiguration {
  concurrency?: {
    'cancel-in-progress'?: boolean
    group?: string
  }
  jobs: Record<string, WorkflowJob>
  name?: string
  on?: Record<string, unknown>
  permissions?: Record<string, string>
}

interface WorkflowJob {
  if?: string
  needs?: string | string[]
  permissions?: Record<string, string>
  steps: WorkflowStep[]
  'timeout-minutes'?: number
}

interface WorkflowStep {
  'continue-on-error'?: boolean
  if?: string
  run?: string
  uses?: string
  with?: Record<string, unknown>
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
    'pnpm run build && node test/browser-load.mjs',
  )
  expect(manifest.devDependencies?.['playwright']).toBeDefined()
  expect(existsSync(resolve(packageRoot, 'test/browser-load.mjs'))).toBe(true)
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

it('packs the published package entry points', () => {
  const packed = JSON.parse(
    execFileSync('pnpm', ['pack', '--dry-run', '--json'], {
      cwd: packageRoot,
      encoding: 'utf8',
    }),
  ) as PackManifest
  const files = packed.files.map(file => file.path)

  expect(packed.name).toBe('fontmin-rs')
  expect(files).toContain('bin/fontmin-rs.mjs')
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
  expect(files.some(file => file.startsWith('test/'))).toBe(false)
})

it('defines repository ci gates', () => {
  const workflowPath = resolve(repositoryRoot, '.github/workflows/ci.yml')

  expect(existsSync(workflowPath)).toBe(true)

  const workflow = readFileSync(workflowPath, 'utf8')
  const workflowConfiguration = parse(workflow) as WorkflowConfiguration
  const checkSteps = workflowConfiguration.jobs['check']?.steps ?? []
  const testJob = workflow.slice(
    workflow.indexOf('  test:'),
    workflow.indexOf('  bench:'),
  )

  expect(workflow).toContain('pnpm run format:check')
  expect(workflow).toContain('pnpm run lint')
  expect(workflow).toContain('targets: wasm32-unknown-unknown')
  expect(workflow).toContain('jetli/wasm-pack-action')
  expect(workflow).toContain('pnpm -C wasm/fontmin run build:wasm')
  expect(workflow).toContain('pnpm run typecheck')
  expect(workflow).toContain('pnpm run test')
  expect(workflow).toContain('pnpm run build')
  expect(workflow).toContain('pnpm --filter fontmin-rs bench')
  expect(workflow).toContain('browser-load:')
  expect(workflow).toContain(
    'pnpm --filter fontmin-rs exec playwright install --with-deps chromium',
  )
  expect(workflow).toContain('pnpm --filter fontmin-rs test:browser')
  expect(workflow).toContain('name: wasm-bindings')
  expect(testJob).toContain('actions/download-artifact')
  expect(testJob).toContain('path: wasm/fontmin/src/generated')
  expect(
    checkSteps.some(
      step =>
        step.uses === 'actions/setup-go@v6' &&
        step.with?.['go-version'] === '1.26.x',
    ),
  ).toBe(true)
  expect(
    checkSteps.some(
      step =>
        step.run ===
        'go install github.com/rhysd/actionlint/cmd/actionlint@v1.7.12',
    ),
  ).toBe(true)
  expect(checkSteps.some(step => step.run === 'actionlint')).toBe(true)
})

it('publishes documentation to the pages branch on main pushes', () => {
  const workflowPath = resolve(
    repositoryRoot,
    '.github/workflows/build-pages.yml',
  )

  expect(existsSync(workflowPath)).toBe(true)

  const workflow = parse(
    readFileSync(workflowPath, 'utf8'),
  ) as WorkflowConfiguration
  const build = workflow.jobs['build']
  const deploy = workflow.jobs['deploy']

  expect(workflow.name).toBe('Build Pages')
  expect(workflow.on).toStrictEqual({ push: { branches: ['main'] } })
  expect(workflow.permissions).toStrictEqual({ contents: 'read' })
  expect(workflow.concurrency).toStrictEqual({
    group: 'build-pages',
    'cancel-in-progress': true,
  })
  expect(Object.keys(workflow.jobs)).toStrictEqual(['build', 'deploy'])

  expect(build).toBeDefined()
  expect(build?.['timeout-minutes']).toBe(30)
  expect(build?.permissions).toBeUndefined()

  const buildSteps = build?.steps ?? []

  expect(buildSteps.map(step => step.uses ?? step.run)).toStrictEqual([
    'actions/checkout@v7',
    'pnpm/action-setup@v6',
    'actions/setup-node@v6',
    'dtolnay/rust-toolchain@stable',
    'jetli/wasm-pack-action@v0.4.0',
    'pnpm install --frozen-lockfile',
    'pnpm -C wasm/fontmin run build:wasm',
    'pnpm run docs:check',
    'actions/upload-artifact@v7',
  ])
  expect(
    buildSteps.every(
      step => step.if === undefined && step['continue-on-error'] === undefined,
    ),
  ).toBe(true)
  expect(buildSteps[0]?.with).toStrictEqual({
    'persist-credentials': false,
  })
  expect(buildSteps[8]?.with).toStrictEqual({
    name: 'pages',
    path: 'docs/.vitepress/dist',
    'if-no-files-found': 'error',
  })

  expect(deploy).toBeDefined()
  expect(deploy?.needs).toBe('build')
  expect(deploy?.permissions).toStrictEqual({ contents: 'write' })
  expect(deploy?.['timeout-minutes']).toBe(30)
  expect(deploy?.if).toBeUndefined()

  const deploySteps = deploy?.steps ?? []

  expect(deploySteps.map(step => step.uses)).toStrictEqual([
    'actions/download-artifact@v7',
    'peaceiris/actions-gh-pages@v4',
  ])
  expect(
    deploySteps.every(
      step => step.if === undefined && step['continue-on-error'] === undefined,
    ),
  ).toBe(true)
  expect(deploySteps[0]?.with).toStrictEqual({
    name: 'pages',
    path: 'docs/.vitepress/dist',
  })
  expect(deploySteps[1]?.with).toStrictEqual({
    github_token: '${{ secrets.GITHUB_TOKEN }}',
    publish_branch: 'pages',
    publish_dir: './docs/.vitepress/dist',
    cname: 'fontmin-rs.ntnyq.dev',
  })
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
    'node ../../scripts/ensure-wasm.mjs',
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
  expect(workflow).toContain('mlugg/setup-zig@v2')
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
  expect(workflow).toContain('mlugg/setup-zig@v2')
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
  expect(workflow).toContain('NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}')
})
