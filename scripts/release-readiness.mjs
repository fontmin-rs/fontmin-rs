import { access, readFile, readdir } from 'node:fs/promises'
import { dirname, join, posix, relative } from 'node:path'
import { pathToFileURL } from 'node:url'

const workspaceRoot = dirname(import.meta.dirname)
const expectedLicense = 'MIT'
const expectedRepositoryUrl = 'git+https://github.com/fontmin-rs/fontmin-rs.git'
const primaryPackageDirectories = [
  'packages/fontmin',
  'napi/fontmin',
  'wasm/fontmin',
]
const platformPackageDirectories = [
  'npm/binding-darwin-arm64',
  'npm/binding-darwin-x64',
  'npm/binding-linux-arm64-gnu',
  'npm/binding-linux-arm64-musl',
  'npm/binding-linux-x64-gnu',
  'npm/binding-linux-x64-musl',
  'npm/binding-win32-arm64-msvc',
  'npm/binding-win32-x64-msvc',
]
const versionArtifacts = [
  {
    path: 'packages/fontmin/src/optimize.ts',
    patterns: [
      /const FONTMIN_VERSION = '(?<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)'/gu,
    ],
  },
  {
    path: 'packages/fontmin/bin/fontmin-rs.mjs',
    patterns: [
      /const FONTMIN_VERSION = '(?<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)'/gu,
    ],
  },
  {
    path: 'napi/fontmin/src-js/bindings.js',
    patterns: [
      /bindingPackageVersion !== '(?<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)'/gu,
      /Native binding package version mismatch, expected (?<version>\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?) but got/gu,
    ],
  },
]

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'))
}

function repositoryUrl(manifest) {
  return typeof manifest.repository === 'string'
    ? manifest.repository
    : manifest.repository?.url
}

async function publishedPackageDirectories(root) {
  const npmRoot = join(root, 'npm')
  const entries = await readdir(npmRoot, { withFileTypes: true })
  const discoveredDirectories = await Promise.all(
    entries
      .filter(entry => entry.isDirectory())
      .map(async entry => {
        const directory = posix.join('npm', entry.name)
        try {
          await access(join(root, directory, 'package.json'))
          return directory
        } catch (error) {
          if (error.code === 'ENOENT') {
            return null
          }
          throw error
        }
      }),
  )
  const availableDirectories = new Set(
    discoveredDirectories.filter(directory => directory !== null),
  )

  for (const directory of platformPackageDirectories) {
    if (!availableDirectories.has(directory)) {
      throw new Error(`missing published package directory ${directory}`)
    }
  }
  for (const directory of availableDirectories) {
    if (!platformPackageDirectories.includes(directory)) {
      throw new Error(`unexpected published package directory ${directory}`)
    }
  }

  return [...primaryPackageDirectories, ...platformPackageDirectories]
}

function rustWorkspaceVersion(cargoManifest) {
  const workspacePackage = cargoManifest.match(
    /\[workspace\.package\](?<workspacePackage>[\s\S]*?)(?=\n\[|$)/u,
  )?.groups?.workspacePackage
  const version = workspacePackage?.match(
    /^version\s*=\s*"(?<version>[^"]+)"/mu,
  )?.groups?.version

  if (version === undefined) {
    throw new Error('Cargo.toml does not define workspace.package.version')
  }

  return version
}

export async function checkReleaseReadiness({
  root = workspaceRoot,
  tag,
} = {}) {
  const directories = await publishedPackageDirectories(root)
  const packages = await Promise.all(
    directories.map(async directory => {
      const manifestPath = join(root, directory, 'package.json')
      const manifest = await readJson(manifestPath)

      return { directory, manifest, manifestPath }
    }),
  )
  const nodePackage = packages.find(
    packageMetadata => packageMetadata.manifest.name === 'fontmin-rs',
  )

  if (nodePackage === undefined) {
    throw new Error('packages/fontmin/package.json must publish fontmin-rs')
  }

  const version = nodePackage.manifest.version
  const issues = []
  const rootManifest = await readJson(join(root, 'package.json'))

  if (
    version === '0.0.0' ||
    !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(version)
  ) {
    issues.push(
      `fontmin-rs version must be a non-placeholder release, got ${version}`,
    )
  }

  if (rootManifest.version !== version) {
    issues.push(
      `root package.json has version ${rootManifest.version}; expected ${version}`,
    )
  }

  for (const packageMetadata of packages) {
    const { manifest } = packageMetadata

    if (manifest.publishConfig?.access !== 'public') {
      issues.push(`${manifest.name} must set publishConfig.access to public`)
    }
    if (manifest.version !== version) {
      issues.push(
        `${manifest.name} has version ${manifest.version}; expected ${version}`,
      )
    }
    if (manifest.license !== expectedLicense) {
      issues.push(`${manifest.name} must set license to ${expectedLicense}`)
    }
    if (repositoryUrl(manifest) !== expectedRepositoryUrl) {
      issues.push(
        `${manifest.name} must set repository.url to ${expectedRepositoryUrl}`,
      )
    }
  }

  const cargoPath = join(root, 'Cargo.toml')
  const cargoVersion = rustWorkspaceVersion(await readFile(cargoPath, 'utf8'))
  if (cargoVersion !== version) {
    issues.push(`Cargo.toml has version ${cargoVersion}; expected ${version}`)
  }

  const changelogPath = join(root, 'CHANGELOG.md')
  const changelog = await readFile(changelogPath, 'utf8')
  if (!changelog.includes(`## [${version}]`)) {
    issues.push(`CHANGELOG.md must contain a ${version} release heading`)
  }

  for (const artifact of versionArtifacts) {
    const contents = await readFile(join(root, artifact.path), 'utf8')
    const embeddedVersions = [
      ...new Set(
        artifact.patterns.flatMap(pattern =>
          [...contents.matchAll(pattern)].map(match => match.groups.version),
        ),
      ),
    ]

    if (embeddedVersions.length !== 1 || embeddedVersions[0] !== version) {
      issues.push(
        `${artifact.path} must only embed ${version}; found ${embeddedVersions.join(', ') || 'no version'}`,
      )
    }
  }

  if (tag !== undefined && tag !== `v${version}`) {
    issues.push(`release tag ${tag} must match v${version}`)
  }

  if (issues.length > 0) {
    throw new Error(`Release readiness failed:\n- ${issues.join('\n- ')}`)
  }

  return {
    packages: packages.map(packageMetadata => ({
      directory: relative(root, dirname(packageMetadata.manifestPath)),
      name: packageMetadata.manifest.name,
    })),
    version,
  }
}

function tagFromArguments(arguments_) {
  const normalizedArguments =
    arguments_[0] === '--' ? arguments_.slice(1) : arguments_

  if (normalizedArguments.length === 0) {
    return
  }
  if (
    normalizedArguments[0] === '--tag' &&
    (normalizedArguments.length < 2 || normalizedArguments[1].startsWith('--'))
  ) {
    throw new Error('--tag requires a value')
  }
  if (normalizedArguments.length !== 2 || normalizedArguments[0] !== '--tag') {
    throw new Error(
      `unknown release metadata arguments: ${arguments_.join(' ')}`,
    )
  }

  const tag = normalizedArguments[1]
  return tag
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(entryPath).href
) {
  try {
    const result = await checkReleaseReadiness({
      tag: tagFromArguments(process.argv.slice(2)),
    })
    process.stdout.write(
      `Release metadata is ready for ${result.version} (${result.packages.length} packages).\n`,
    )
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : error}\n`)
    process.exitCode = 1
  }
}
