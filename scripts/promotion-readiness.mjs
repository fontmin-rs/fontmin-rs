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

function assertPassedEvidence(evidence) {
  for (const name of requiredEvidence) {
    const item = evidence?.[name]

    if (item?.status !== 'passed') {
      throw new Error(`1.0 readiness evidence ${name} has not passed`)
    }
  }
}

export function isVersionOnlyRuntimeDiff(
  diff,
  { fromVersion = '1.0.0-rc.1', toVersion = '1.0.0' } = {},
) {
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
}) {
  if (audit?.schemaVersion !== 1 || audit.status !== 'passed') {
    throw new Error('1.0 readiness audit must use schema version 1 and pass')
  }
  if (audit.candidate?.tag !== 'v1.0.0-rc.1') {
    throw new Error('1.0 readiness audit must reference v1.0.0-rc.1')
  }

  for (const category of requiredCategories) {
    const issues = audit.categories?.[category]

    if (
      !Array.isArray(issues?.unresolvedP0) ||
      !Array.isArray(issues?.unresolvedP1)
    ) {
      throw new TypeError(`1.0 readiness category ${category} is incomplete`)
    }
    if (issues.unresolvedP0.length > 0 || issues.unresolvedP1.length > 0) {
      throw new Error(`1.0 readiness category ${category} has P0/P1 issues`)
    }
  }

  assertPassedEvidence(audit.evidence)

  if (
    report?.schemaVersion !== 1 ||
    report.source?.type !== 'npm-registry' ||
    report.source?.version !== audit.candidate.version ||
    report.summary?.failed !== 0 ||
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
  execute = executeFile,
  root = workspaceRoot,
} = {}) {
  const auditPath = join(root, 'audits/1.0-readiness.json')
  const audit = JSON.parse(await readFile(auditPath, 'utf8'))
  const reportPath = join(root, audit.evidence.registryCompatibility.path)
  const report = JSON.parse(await readFile(reportPath, 'utf8'))
  const { stdout: candidateCommit } = await execute(
    'git',
    ['rev-list', '-n', '1', audit.candidate.tag],
    { cwd: root },
  )

  if (candidateCommit.trim() !== audit.candidate.commit) {
    throw new Error('1.0 readiness audit does not match the RC tag')
  }

  const { stdout: changedPaths } = await execute(
    'git',
    ['diff', '--name-only', audit.candidate.tag, '--', ...runtimePaths],
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
      ['diff', '--unified=0', audit.candidate.tag, '--', path],
      { cwd: root },
    )

    if (isVersionOnlyRuntimeDiff(diff)) {
      versionOnlyChanges += 1
    } else {
      invalidRuntimePaths.push(path)
    }
  }

  validatePromotionReadiness({
    audit,
    changedRuntimePaths: invalidRuntimePaths,
    report,
  })

  return {
    candidate: audit.candidate.version,
    runtimeChanges: invalidRuntimePaths.length,
    status: 'passed',
    versionOnlyChanges,
  }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const result = await promotionReadiness()

  console.log(JSON.stringify(result))
}
