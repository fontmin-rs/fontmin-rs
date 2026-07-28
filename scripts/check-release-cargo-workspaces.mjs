import { execFileSync } from 'node:child_process'
import { readFileSync } from 'node:fs'
import { pathToFileURL } from 'node:url'

const cargoCheckArguments = [
  ['check', '--workspace'],
  ['check', '--manifest-path', 'fuzz/Cargo.toml'],
]

export function checkCargoVersionBumpSafety(cargoManifest) {
  const workspaceVersion = cargoManifest.match(
    /\[workspace\.package\][\s\S]*?^version\s*=\s*"(?<version>[^"]+)"/mu,
  )?.groups?.version
  const dependencies = cargoManifest.match(
    /\[workspace\.dependencies\](?<dependencies>[\s\S]*?)(?=\n\[|$)/u,
  )?.groups?.dependencies

  assertManifestSection(workspaceVersion, 'workspace.package version')
  assertManifestSection(dependencies, 'workspace.dependencies')

  const unsafeDependencies = [
    ...dependencies.matchAll(
      /^(?<name>[A-Za-z0-9_-]+)\s*=\s*"(?<version>[^"]+)"$/gmu,
    ),
  ]
    .filter(match => match.groups?.version === workspaceVersion)
    .map(match => match.groups?.name)

  if (unsafeDependencies.length > 0) {
    throw new Error(
      `external Cargo dependencies must not exactly match the workspace version because the release bump replaces every matching string: ${unsafeDependencies.join(', ')}`,
    )
  }
}

function assertManifestSection(value, label) {
  if (value === undefined) {
    throw new Error(`Cargo.toml does not define ${label}`)
  }
}

export function checkReleaseCargoWorkspaces(execute = execFileSync) {
  checkCargoVersionBumpSafety(
    readFileSync(new URL('../Cargo.toml', import.meta.url), 'utf8'),
  )

  for (const cargoArguments of cargoCheckArguments) {
    execute('cargo', cargoArguments, { stdio: 'inherit' })
  }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(entryPath).href
) {
  try {
    checkReleaseCargoWorkspaces()
  } catch (error) {
    process.stderr.write(
      `Release Cargo workspace checks failed: ${
        error instanceof Error ? error.message : error
      }\n`,
    )
    process.exitCode = 1
  }
}
