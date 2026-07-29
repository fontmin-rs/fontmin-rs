/**
 * Canonical native release layout derived from `napi.targets`.
 *
 * Artifact copy, package validation, smoke tests, and release workflows must
 * consume this interface instead of maintaining independent platform maps.
 */
import { access, readFile, readdir } from 'node:fs/promises'
import { dirname, join, posix, resolve } from 'node:path'

const workspaceRoot = dirname(import.meta.dirname)
const bindingManifestPath = 'napi/fontmin/package.json'
const binaryName = 'fontmin_rs'
const packageName = '@fontmin-rs/binding'
const architectureByTarget = new Map([
  ['aarch64', 'arm64'],
  ['x86_64', 'x64'],
])

function singleValueMatches(actual, expected) {
  return Array.isArray(actual) && actual.length === 1 && actual[0] === expected
}

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'))
}

export function nativeTargetToReleaseEntry(target) {
  const [targetArchitecture, vendor, operatingSystem, abi] = target.split('-')
  const cpu = architectureByTarget.get(targetArchitecture)

  if (cpu === undefined) {
    throw new Error(`Unsupported native target architecture: ${target}`)
  }

  let libc
  let os
  let platform

  if (vendor === 'apple' && operatingSystem === 'darwin' && abi === undefined) {
    os = 'darwin'
    platform = `${os}-${cpu}`
  } else if (
    vendor === 'pc' &&
    operatingSystem === 'windows' &&
    abi === 'msvc'
  ) {
    os = 'win32'
    platform = `${os}-${cpu}-${abi}`
  } else if (
    vendor === 'unknown' &&
    operatingSystem === 'linux' &&
    (abi === 'gnu' || abi === 'musl')
  ) {
    os = 'linux'
    libc = abi === 'gnu' ? 'glibc' : 'musl'
    platform = `${os}-${cpu}-${abi}`
  } else {
    throw new Error(`Unsupported native target layout: ${target}`)
  }

  const packageDirectory = `binding-${platform}`

  return {
    artifactName: `${binaryName}.${platform}.node`,
    cpu,
    directory: posix.join('npm', packageDirectory),
    libc,
    name: `${packageName}-${platform}`,
    os,
    packageDirectory,
    platform,
    target,
  }
}

export async function readNativeReleaseLayout({ root = workspaceRoot } = {}) {
  const manifest = await readJson(join(root, bindingManifestPath))
  const targets = manifest.napi?.targets

  if (!Array.isArray(targets) || targets.length === 0) {
    throw new Error(`${bindingManifestPath} must define napi.targets`)
  }

  const entries = targets.map(target => nativeTargetToReleaseEntry(target))
  const uniqueTargets = new Set(entries.map(entry => entry.target))
  const uniquePlatforms = new Set(entries.map(entry => entry.platform))

  if (uniqueTargets.size !== entries.length) {
    throw new Error(`${bindingManifestPath} contains duplicate napi.targets`)
  }
  if (uniquePlatforms.size !== entries.length) {
    throw new Error(
      `${bindingManifestPath} maps multiple napi.targets to one platform`,
    )
  }

  return { bindingManifest: manifest, entries }
}

export function findNativeReleaseEntry(entries, { arch, libc, platform }) {
  const matches = entries.filter(entry => {
    if (entry.cpu !== arch || entry.os !== platform) {
      return false
    }

    return platform !== 'linux' || entry.libc === libc
  })

  if (matches.length !== 1) {
    const runtime = [platform, arch, libc].filter(Boolean).join('-')
    throw new Error(`Unsupported native release runtime: ${runtime}`)
  }

  return matches[0]
}

export async function validateNativeReleaseLayout({
  requireArtifacts = false,
  root = workspaceRoot,
} = {}) {
  const { bindingManifest, entries } = await readNativeReleaseLayout({ root })
  const issues = []
  const expectedPackageNames = new Set(entries.map(entry => entry.name))
  const optionalDependencies = bindingManifest.optionalDependencies ?? {}
  const configuredPackageNames = Object.keys(optionalDependencies).filter(
    name => name.startsWith(`${packageName}-`),
  )

  for (const entry of entries) {
    if (optionalDependencies[entry.name] !== 'workspace:*') {
      issues.push(
        `${bindingManifestPath} must declare ${entry.name} as workspace:*`,
      )
    }
  }
  for (const configuredName of configuredPackageNames) {
    if (!expectedPackageNames.has(configuredName)) {
      issues.push(
        `${bindingManifestPath} declares unexpected native package ${configuredName}`,
      )
    }
  }

  const npmRoot = join(root, 'npm')
  const npmEntries = await readdir(npmRoot, { withFileTypes: true })
  const packageDirectories = new Set(
    npmEntries
      .filter(entry => entry.isDirectory())
      .map(entry => posix.join('npm', entry.name)),
  )
  const expectedDirectories = new Set(entries.map(entry => entry.directory))

  for (const entry of entries) {
    if (!packageDirectories.has(entry.directory)) {
      issues.push(`missing published package directory ${entry.directory}`)
      continue
    }

    const packageRoot = join(root, entry.directory)
    const manifestPath = join(packageRoot, 'package.json')
    let manifest

    try {
      manifest = await readJson(manifestPath)
    } catch (error) {
      if (error?.code === 'ENOENT') {
        issues.push(`${entry.directory} must contain package.json`)
        continue
      }
      throw error
    }

    if (manifest.name !== entry.name) {
      issues.push(`${entry.directory} must publish ${entry.name}`)
    }
    if (manifest.main !== entry.artifactName) {
      issues.push(`${entry.name} main must be ${entry.artifactName}`)
    }
    if (
      !Array.isArray(manifest.files) ||
      !manifest.files.includes(entry.artifactName)
    ) {
      issues.push(`${entry.name} files must include ${entry.artifactName}`)
    }
    if (!singleValueMatches(manifest.os, entry.os)) {
      issues.push(`${entry.name} os must be ${entry.os}`)
    }
    if (!singleValueMatches(manifest.cpu, entry.cpu)) {
      issues.push(`${entry.name} cpu must be ${entry.cpu}`)
    }
    if (
      (entry.libc === undefined && manifest.libc !== undefined) ||
      (entry.libc !== undefined &&
        !singleValueMatches(manifest.libc, entry.libc))
    ) {
      issues.push(`${entry.name} libc must be ${entry.libc ?? 'omitted'}`)
    }

    if (requireArtifacts) {
      try {
        await access(join(packageRoot, entry.artifactName))
      } catch (error) {
        if (error?.code === 'ENOENT') {
          issues.push(`${entry.name} is missing ${entry.artifactName}`)
        } else {
          throw error
        }
      }
    }
  }

  for (const directory of packageDirectories) {
    if (!expectedDirectories.has(directory)) {
      issues.push(`unexpected published package directory ${directory}`)
    }
  }

  if (issues.length > 0) {
    throw new Error(
      `Native release layout is invalid:\n- ${issues.join('\n- ')}`,
    )
  }

  return entries
}

const entrypoint = process.argv[1] && resolve(process.argv[1])
if (entrypoint === import.meta.filename) {
  const command = process.argv[2]

  if (command !== 'verify' && command !== 'verify-artifacts') {
    throw new Error(
      'Usage: node scripts/native-release-layout.mjs <verify|verify-artifacts>',
    )
  }

  const entries = await validateNativeReleaseLayout({
    requireArtifacts: command === 'verify-artifacts',
  })
  console.log(
    `Verified ${entries.length} native release packages${
      command === 'verify-artifacts' ? ' and artifacts' : ''
    }.`,
  )
}
