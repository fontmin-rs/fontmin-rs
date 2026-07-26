import { createHash } from 'node:crypto'
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises'
import { dirname, join, posix, relative, resolve, sep } from 'node:path'
import { pathToFileURL } from 'node:url'
import { fuzzOperation } from './fuzz-operations.mjs'

const workspaceRoot = dirname(import.meta.dirname)
const defaultArtifactDirectory = 'fuzz/minimized/public_api'
const defaultRegressionDirectory = 'fuzz/regressions/public_api'

function repositoryPath(root, path) {
  return relative(root, path).split(sep).join(posix.sep)
}

async function readManifest(path) {
  const manifest = JSON.parse(await readFile(path, 'utf8'))

  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.cases)) {
    throw new Error(`${path} must use schema version 1`)
  }

  return manifest
}

function parseArguments(args) {
  const values = {}

  for (let index = 0; index < args.length; index += 2) {
    const flag = args[index]
    const value = args[index + 1]

    if (value === undefined) {
      throw new Error(`missing value for ${flag}`)
    }
    if (
      !['--artifacts', '--commit', '--regressions', '--run-url'].includes(flag)
    ) {
      throw new Error(`unexpected argument: ${flag}`)
    }

    values[flag.slice(2)] = value
  }

  return values
}

export async function promoteFuzzArtifacts({
  artifactsDirectory = defaultArtifactDirectory,
  commit,
  regressionsDirectory = defaultRegressionDirectory,
  root = workspaceRoot,
  runUrl,
}) {
  if (!/^[\da-f]{40}$/u.test(commit ?? '')) {
    throw new Error('fuzz promotion requires a full 40-character commit SHA')
  }
  if (typeof runUrl !== 'string' || !runUrl.startsWith('https://github.com/')) {
    throw new Error('fuzz promotion requires a GitHub Actions run URL')
  }

  const artifactRoot = resolve(root, artifactsDirectory)
  const regressionRoot = resolve(root, regressionsDirectory)
  const manifestPath = join(regressionRoot, 'manifest.json')
  const artifactEntries = await readdir(artifactRoot, { withFileTypes: true })
  const entries = artifactEntries
    .filter(entry => entry.isFile())
    .toSorted((left, right) => left.name.localeCompare(right.name))

  if (entries.length === 0) {
    throw new Error(`no minimized fuzz artifacts found in ${artifactRoot}`)
  }

  await mkdir(regressionRoot, { recursive: true })
  const manifest = await readManifest(manifestPath)
  const casesByDigest = new Map(
    manifest.cases.map(testCase => [testCase.sha256, testCase]),
  )
  const promoted = []

  for (const entry of entries) {
    const contents = await readFile(join(artifactRoot, entry.name))

    if (contents.length === 0) {
      throw new Error(`${entry.name} does not include an operation byte`)
    }

    const sha256 = createHash('sha256').update(contents).digest('hex')
    const fileName = `crash-${sha256}.bin`
    const outputPath = join(regressionRoot, fileName)
    const path = repositoryPath(root, outputPath)
    const existing = casesByDigest.get(sha256)

    if (existing === undefined) {
      const testCase = {
        operation: fuzzOperation(contents[0]),
        path,
        sha256,
        source: {
          commit,
          kind: 'fuzz',
          license: 'NOASSERTION',
          runUrl,
        },
      }

      await writeFile(outputPath, contents)
      await writeFile(`${outputPath}.sha256`, `${sha256}  ${path}\n`)
      casesByDigest.set(sha256, testCase)
      promoted.push(testCase)
    }
  }

  const cases = [...casesByDigest.values()].toSorted((left, right) =>
    left.path.localeCompare(right.path),
  )

  await writeFile(
    manifestPath,
    `${JSON.stringify({ cases, schemaVersion: 1 }, null, 2)}\n`,
  )

  return { promoted, total: cases.length }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const args = parseArguments(process.argv.slice(2))
  const result = await promoteFuzzArtifacts({
    artifactsDirectory: args.artifacts,
    commit: args.commit ?? process.env.GITHUB_SHA,
    regressionsDirectory: args.regressions,
    runUrl:
      args['run-url'] ??
      (process.env.GITHUB_SERVER_URL &&
      process.env.GITHUB_REPOSITORY &&
      process.env.GITHUB_RUN_ID
        ? `${process.env.GITHUB_SERVER_URL}/${process.env.GITHUB_REPOSITORY}/actions/runs/${process.env.GITHUB_RUN_ID}`
        : undefined),
  })

  console.log(
    `Promoted ${result.promoted.length} minimized fuzz artifact(s); ${result.total} permanent regression(s) total.`,
  )
}
