import { readFile, readdir, stat, writeFile, mkdir } from 'node:fs/promises'
import { arch, platform } from 'node:os'
import { dirname, join, relative, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const workspaceRoot = dirname(import.meta.dirname)
const policyRelativePath = 'audits/release-policy.json'

function parseCargoPackages(source) {
  return source
    .split('[[package]]')
    .slice(1)
    .map(block => ({
      name: block.match(/^name = "(?<name>[^"]+)"$/mu)?.groups?.name,
      version: block.match(/^version = "(?<version>[^"]+)"$/mu)?.groups
        ?.version,
    }))
    .filter(entry => entry.name !== undefined && entry.version !== undefined)
}

function findDuplicateDependencies(source) {
  const versionsByName = new Map()

  for (const dependency of parseCargoPackages(source)) {
    const versions = versionsByName.get(dependency.name) ?? new Set()

    versions.add(dependency.version)
    versionsByName.set(dependency.name, versions)
  }

  return [...versionsByName]
    .filter(([, versions]) => versions.size > 1)
    .map(([name, versions]) => ({
      name,
      versions: [...versions].toSorted(),
    }))
    .toSorted((left, right) => left.name.localeCompare(right.name))
}

function assertPolicy(policy) {
  if (
    policy.schemaVersion !== 1 ||
    !Array.isArray(policy.duplicateDependencies) ||
    !Array.isArray(policy.vendoredPatches) ||
    !Array.isArray(policy.artifacts)
  ) {
    throw new Error(`${policyRelativePath} must use schema version 1`)
  }
}

function validateDuplicatePolicy(actual, expected) {
  const actualByName = new Map(actual.map(entry => [entry.name, entry]))
  const expectedByName = new Map(expected.map(entry => [entry.name, entry]))
  const violations = []

  for (const entry of actual) {
    const policy = expectedByName.get(entry.name)

    if (policy === undefined) {
      violations.push(`duplicated dependency ${entry.name} has no decision`)
    } else if (
      JSON.stringify(entry.versions) !==
      JSON.stringify([...policy.versions].toSorted())
    ) {
      violations.push(
        `${entry.name} resolves ${entry.versions.join(', ')}; policy records ${policy.versions.join(', ')}`,
      )
    }
  }

  for (const entry of expected) {
    if (!actualByName.has(entry.name)) {
      violations.push(
        `${entry.name} is no longer duplicated; remove its retained decision`,
      )
    }
    if (
      entry.owner.length === 0 ||
      entry.decision.length === 0 ||
      entry.replacementCondition.length === 0
    ) {
      violations.push(
        `${entry.name} has incomplete ownership or replacement data`,
      )
    }
  }

  return violations
}

async function loadPolicy(root) {
  const policy = JSON.parse(
    await readFile(join(root, policyRelativePath), 'utf8'),
  )

  assertPolicy(policy)
  return policy
}

export async function auditDependencyPolicy({ root = workspaceRoot } = {}) {
  const [lockSource, cargoManifest, policy] = await Promise.all([
    readFile(join(root, 'Cargo.lock'), 'utf8'),
    readFile(join(root, 'Cargo.toml'), 'utf8'),
    loadPolicy(root),
  ])
  const actualDuplicates = findDuplicateDependencies(lockSource)
  const violations = validateDuplicatePolicy(
    actualDuplicates,
    policy.duplicateDependencies,
  )

  for (const entry of policy.vendoredPatches) {
    if (
      entry.owner.length === 0 ||
      entry.decision.length === 0 ||
      entry.replacementCondition.length === 0 ||
      !entry.upstream.startsWith('https://')
    ) {
      violations.push(
        `${entry.crate} has incomplete ownership or replacement data`,
      )
      continue
    }

    const notes = await readFile(join(root, entry.notes), 'utf8')
    const patchExpression = `${entry.crate} = { path = "${entry.path}"`

    if (!cargoManifest.includes(patchExpression)) {
      violations.push(`${entry.crate} is not pinned to ${entry.path}`)
    }
    if (!notes.includes(entry.removalMarker)) {
      violations.push(
        `${entry.notes} does not state the recorded removal condition`,
      )
    }
  }

  if (violations.length > 0) {
    throw new Error(`dependency policy failed:\n${violations.join('\n')}`)
  }

  return {
    duplicates: policy.duplicateDependencies,
    schemaVersion: 1,
    status: 'passed',
    vendored: policy.vendoredPatches,
  }
}

async function firstExistingPath(root, paths) {
  for (const path of paths) {
    try {
      const metadata = await stat(join(root, path))

      if (metadata.isFile()) {
        return { bytes: metadata.size, path }
      }
    } catch (error) {
      if (error?.code !== 'ENOENT') {
        throw error
      }
    }
  }

  throw new Error(`none of these artifact paths exist: ${paths.join(', ')}`)
}

async function singleMatchingPath(root, directory, suffix) {
  const entries = await readdir(join(root, directory), {
    withFileTypes: true,
  })
  const matches = entries.filter(
    entry => entry.isFile() && entry.name.endsWith(suffix),
  )

  if (matches.length !== 1) {
    throw new Error(
      `${directory} must contain exactly one ${suffix} artifact; found ${matches.length}`,
    )
  }

  const path = join(directory, matches[0].name)
  const metadata = await stat(join(root, path))

  return { bytes: metadata.size, path }
}

async function measureArtifact(root, artifact) {
  const measurement =
    artifact.paths === undefined
      ? await singleMatchingPath(root, artifact.directory, artifact.suffix)
      : await firstExistingPath(root, artifact.paths)
  const passed = measurement.bytes <= artifact.maxBytes

  return {
    bytes: measurement.bytes,
    id: artifact.id,
    maxBytes: artifact.maxBytes,
    path: relative(root, join(root, measurement.path)),
    status: passed ? 'passed' : 'failed',
    violations: passed
      ? []
      : [
          `${artifact.id} is ${measurement.bytes} bytes; budget is ${artifact.maxBytes} bytes`,
        ],
  }
}

export async function auditArtifacts({
  generatedAt = new Date().toISOString(),
  output = join(workspaceRoot, 'audits/artifact-current.json'),
  root = workspaceRoot,
} = {}) {
  const [policy, packageManifest] = await Promise.all([
    loadPolicy(root),
    readFile(join(root, 'package.json'), 'utf8').then(JSON.parse),
  ])
  const artifacts = []

  for (const artifact of policy.artifacts) {
    try {
      artifacts.push(await measureArtifact(root, artifact))
    } catch (error) {
      artifacts.push({
        bytes: null,
        id: artifact.id,
        maxBytes: artifact.maxBytes,
        path: null,
        status: 'failed',
        violations: [
          `${artifact.id} measurement failed: ${error instanceof Error ? error.message : String(error)}`,
        ],
      })
    }
  }

  const violations = artifacts.flatMap(artifact => artifact.violations)
  const report = {
    artifacts,
    environment: {
      arch: arch(),
      node: process.version,
      os: platform(),
    },
    generatedAt,
    schemaVersion: 1,
    status: violations.length === 0 ? 'passed' : 'failed',
    version: packageManifest.version,
  }

  await mkdir(dirname(output), { recursive: true })
  await writeFile(output, `${JSON.stringify(report, null, 2)}\n`)

  if (violations.length > 0) {
    throw new Error(`artifact budgets failed:\n${violations.join('\n')}`)
  }

  return report
}

function parseArguments(args) {
  if (args.length === 1 && args[0] === '--dependencies-only') {
    return { dependenciesOnly: true }
  }
  if (args.length === 0) {
    return {
      dependenciesOnly: false,
      output: join(workspaceRoot, 'audits/artifact-current.json'),
    }
  }
  if (args.length === 2 && args[0] === '--output') {
    return {
      dependenciesOnly: false,
      output: resolve(workspaceRoot, args[1]),
    }
  }

  throw new Error(
    'usage: dependency-artifact-audit.mjs [--dependencies-only | --output <report.json>]',
  )
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const options = parseArguments(process.argv.slice(2))
  const dependencyReport = await auditDependencyPolicy()

  if (options.dependenciesOnly) {
    console.log(
      `Dependency policy passed for ${dependencyReport.duplicates.length} duplicate groups and ${dependencyReport.vendored.length} vendored patches.`,
    )
  } else {
    const artifactReport = await auditArtifacts({ output: options.output })

    console.log(
      `Artifact budgets passed for ${artifactReport.artifacts.length} release artifacts.`,
    )
  }
}
