import { existsSync, readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'

interface PackageManifest {
  devDependencies?: Record<string, string>
  optionalDependencies?: Record<string, string>
  private?: boolean
  publishConfig?: {
    access?: string
  }
  scripts?: Record<string, string>
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

it('defines repository ci gates', () => {
  const workflowPath = resolve(repositoryRoot, '.github/workflows/ci.yml')

  expect(existsSync(workflowPath)).toBe(true)

  const workflow = readFileSync(workflowPath, 'utf8')

  expect(workflow).toContain('pnpm run format:check')
  expect(workflow).toContain('pnpm run lint')
  expect(workflow).toContain('pnpm run typecheck')
  expect(workflow).toContain('pnpm run test')
  expect(workflow).toContain('pnpm run build')
  expect(workflow).toContain('pnpm --filter fontmin-rs bench')
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
    expect(platformManifest.os).toEqual(platformPackage.os)
    expect(platformManifest.cpu).toEqual(platformPackage.cpu)
    expect(platformManifest.libc).toEqual(platformPackage.libc)
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
    'napi artifacts --npm-dir ../../npm --output-dir src-js',
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
  expect(workflow).toContain(
    'pnpm --filter @fontmin-rs/binding build --target ${{ matrix.target }}',
  )
  expect(workflow).toContain('pnpm --filter @fontmin-rs/binding artifacts')
  expect(workflow).toContain('actions/upload-artifact')
})
