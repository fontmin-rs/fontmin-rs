import { readFile, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'

const workspaceRoot = dirname(import.meta.dirname)

export function extractReleaseNotes(changelog, version) {
  const lines = changelog.replaceAll('\r\n', '\n').split('\n')
  const heading = `## [${version}]`
  const start = lines.findIndex(
    line => line === heading || line.startsWith(`${heading} - `),
  )

  if (start === -1) {
    throw new Error(
      `CHANGELOG.md does not contain release notes for ${version}`,
    )
  }

  const nextHeading = lines.findIndex(
    (line, index) => index > start && /^## \[[^\]]+\](?: |$)/u.test(line),
  )
  const comparisonLinks = lines.findIndex(
    (line, index) => index > start && /^\[[^\]]+\]:\s+\S/u.test(line),
  )
  const sectionEndIndexes = [nextHeading, comparisonLinks].filter(
    index => index !== -1,
  )
  const end =
    sectionEndIndexes.length === 0
      ? lines.length
      : Math.min(...sectionEndIndexes)
  const notes = lines
    .slice(start + 1, end)
    .join('\n')
    .trim()

  if (notes.length === 0) {
    throw new Error(`CHANGELOG.md release notes for ${version} are empty`)
  }

  return notes
}

export async function writeReleaseNotes({
  output,
  root = workspaceRoot,
  version,
} = {}) {
  if (output === undefined) {
    throw new Error('--output requires a value')
  }

  const releaseVersion =
    version ??
    JSON.parse(await readFile(join(root, 'package.json'), 'utf8')).version
  const changelog = await readFile(join(root, 'CHANGELOG.md'), 'utf8')
  const notes = extractReleaseNotes(changelog, releaseVersion)
  const outputPath = resolve(root, output)

  await writeFile(outputPath, `${notes}\n`)

  return { output: outputPath, version: releaseVersion }
}

function parseArguments(arguments_) {
  const normalizedArguments =
    arguments_[0] === '--' ? arguments_.slice(1) : arguments_

  if (
    normalizedArguments.length !== 2 ||
    normalizedArguments[0] !== '--output' ||
    normalizedArguments[1].startsWith('--')
  ) {
    throw new Error('usage: release-notes.mjs --output <path>')
  }

  return { output: normalizedArguments[1] }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  try {
    const result = await writeReleaseNotes(
      parseArguments(process.argv.slice(2)),
    )
    process.stdout.write(
      `Wrote GitHub release notes for ${result.version} to ${result.output}.\n`,
    )
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : error}\n`)
    process.exitCode = 1
  }
}
