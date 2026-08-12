import { execFile } from 'node:child_process'
import { readFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)
const runtimePaths = [
  'apps/fontmin/src',
  'crates',
  'napi/fontmin/src',
  'napi/fontmin/src-js',
  'packages/fontmin/src',
  'wasm/fontmin-core/src',
  'wasm/fontmin/src',
]
const versionOnlyRuntimePaths = new Set([
  'napi/fontmin/src-js/bindings.js',
  'packages/fontmin/src/cli.mjs',
  'packages/fontmin/src/optimize-storage.ts',
])
const requiredCategories = [
  'correctness',
  'packaging',
  'performance',
  'security',
]
const requiredEvidence = [
  'boundaryRegression',
  'mainCi',
  'npmRegistry',
  'registryCompatibility',
  'release',
  'releaseArtifacts',
]
const releaseVersionPattern =
  /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-(?<prerelease>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/u

function assertPassedEvidence(evidence) {
  for (const name of requiredEvidence) {
    const item = evidence?.[name]

    if (item?.status !== 'passed') {
      throw new Error(`promotion readiness evidence ${name} has not passed`)
    }
  }
}

function versionFromStableTag(tag) {
  if (typeof tag !== 'string' || !tag.startsWith('v')) {
    throw new Error(`stable promotion tag ${tag ?? '<missing>'} is invalid`)
  }

  const version = tag.slice(1)
  const match = version.match(releaseVersionPattern)

  if (match === null || match.groups?.prerelease !== undefined) {
    throw new Error(`stable promotion tag ${tag} is invalid`)
  }

  return version
}

function validatePromotionCycle(audit, targetTag) {
  if (audit?.schemaVersion !== 1 || audit.status !== 'passed') {
    throw new Error(
      'promotion readiness audit must use schema version 1 and pass',
    )
  }

  const candidate = audit.candidate
  const target = audit.target
  const candidatePrerelease = candidate?.version
    ?.match(releaseVersionPattern)
    ?.groups?.prerelease?.split('.')[0]

  if (
    typeof candidate?.version !== 'string' ||
    candidate.tag !== `v${candidate.version}` ||
    candidatePrerelease?.toLowerCase() !== 'rc'
  ) {
    throw new Error('promotion readiness audit must reference an RC tag')
  }
  if (
    typeof target?.version !== 'string' ||
    target.tag !== `v${target.version}` ||
    versionFromStableTag(target.tag) !== target.version
  ) {
    throw new Error('promotion readiness audit must define a stable target')
  }
  if (target.tag !== targetTag) {
    throw new Error(`promotion audit targets ${target.tag}, not ${targetTag}`)
  }
  if (candidate.version.split('-')[0] !== target.version) {
    throw new Error(
      `promotion candidate ${candidate.version} does not match ${target.version}`,
    )
  }

  return { candidate, target }
}

export function isVersionOnlyRuntimeDiff(diff, { fromVersion, toVersion }) {
  const lines = diff.split(/\r?\n/u)
  const removed = lines
    .filter(line => line.startsWith('-') && !line.startsWith('---'))
    .map(line => line.slice(1))
  const added = lines
    .filter(line => line.startsWith('+') && !line.startsWith('+++'))
    .map(line => line.slice(1))

  return (
    removed.length > 0 &&
    removed.length === added.length &&
    removed.every(
      (line, index) =>
        line.includes(fromVersion) &&
        line.replaceAll(fromVersion, toVersion) === added[index],
    )
  )
}

export function validatePromotionReadiness({
  audit,
  changedRuntimePaths = [],
  report,
  targetTag,
}) {
  const { candidate } = validatePromotionCycle(audit, targetTag)

  for (const category of requiredCategories) {
    const issues = audit.categories?.[category]

    if (
      !Array.isArray(issues?.unresolvedP0) ||
      !Array.isArray(issues?.unresolvedP1)
    ) {
      throw new TypeError(
        `promotion readiness category ${category} is incomplete`,
      )
    }
    if (issues.unresolvedP0.length > 0 || issues.unresolvedP1.length > 0) {
      throw new Error(
        `promotion readiness category ${category} has P0/P1 issues`,
      )
    }
  }

  assertPassedEvidence(audit.evidence)

  if (
    report?.schemaVersion !== 1 ||
    report.source?.type !== 'npm-registry' ||
    report.source?.version !== candidate.version ||
    report.summary?.failed !== 0 ||
    !Number.isInteger(report.summary?.total) ||
    report.summary.total <= 0 ||
    report.summary?.passed !== report.summary?.total
  ) {
    throw new Error('published RC compatibility evidence is incomplete')
  }

  if (changedRuntimePaths.length > 0) {
    throw new Error(
      `runtime sources changed after the RC: ${changedRuntimePaths.join(', ')}`,
    )
  }
}

export async function promotionReadiness({
  auditPath,
  execute = executeFile,
  root = workspaceRoot,
  tag,
} = {}) {
  const targetVersion = versionFromStableTag(tag)
  const resolvedAuditPath = resolve(
    root,
    auditPath ?? join('audits', `${targetVersion}-readiness.json`),
  )
  const audit = JSON.parse(await readFile(resolvedAuditPath, 'utf8'))
  const { candidate, target } = validatePromotionCycle(audit, tag)
  const reportPath = join(root, audit.evidence.registryCompatibility.path)
  const report = JSON.parse(await readFile(reportPath, 'utf8'))
  const { stdout: candidateCommit } = await execute(
    'git',
    ['rev-list', '-n', '1', candidate.tag],
    { cwd: root },
  )

  if (candidateCommit.trim() !== candidate.commit) {
    throw new Error('promotion readiness audit does not match the RC tag')
  }

  const { stdout: changedPaths } = await execute(
    'git',
    ['diff', '--name-only', candidate.tag, '--', ...runtimePaths],
    { cwd: root },
  )
  const changedRuntimePaths = changedPaths
    .split(/\r?\n/u)
    .filter(Boolean)
    .toSorted()
  const invalidRuntimePaths = []
  let versionOnlyChanges = 0

  for (const path of changedRuntimePaths) {
    if (!versionOnlyRuntimePaths.has(path)) {
      invalidRuntimePaths.push(path)
      continue
    }

    const { stdout: diff } = await execute(
      'git',
      ['diff', '--unified=0', candidate.tag, '--', path],
      { cwd: root },
    )

    if (
      isVersionOnlyRuntimeDiff(diff, {
        fromVersion: candidate.version,
        toVersion: target.version,
      })
    ) {
      versionOnlyChanges += 1
    } else {
      invalidRuntimePaths.push(path)
    }
  }

  validatePromotionReadiness({
    audit,
    changedRuntimePaths: invalidRuntimePaths,
    report,
    targetTag: tag,
  })

  return {
    candidate: candidate.version,
    runtimeChanges: invalidRuntimePaths.length,
    status: 'passed',
    target: target.version,
    versionOnlyChanges,
  }
}

function tagFromArguments(arguments_) {
  const normalizedArguments =
    arguments_[0] === '--' ? arguments_.slice(1) : arguments_

  if (
    normalizedArguments.length !== 2 ||
    normalizedArguments[0] !== '--tag' ||
    normalizedArguments[1].startsWith('--')
  ) {
    throw new Error('usage: promotion-readiness.mjs --tag v<stable-version>')
  }

  return normalizedArguments[1]
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  try {
    const result = await promotionReadiness({
      tag: tagFromArguments(process.argv.slice(2)),
    })
    process.stdout.write(`${JSON.stringify(result)}\n`)
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : error}\n`)
    process.exitCode = 1
  }
}
