import { readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { pathToFileURL } from 'node:url'

const workspaceRoot = dirname(import.meta.dirname)
const releaseVersionPattern =
  /^(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-(?<prerelease>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/u

export function distTagForVersion(version) {
  const prerelease = version.match(releaseVersionPattern)?.groups?.prerelease

  if (!releaseVersionPattern.test(version)) {
    throw new Error(`invalid release version ${version}`)
  }
  if (prerelease === undefined) {
    return 'latest'
  }

  const channel = prerelease.split('.')[0].toLowerCase()
  return /^[a-z][a-z0-9-]*$/u.test(channel) ? channel : 'next'
}

async function main() {
  const manifest = JSON.parse(
    await readFile(join(workspaceRoot, 'package.json'), 'utf8'),
  )
  process.stdout.write(`${distTagForVersion(manifest.version)}\n`)
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(entryPath).href
) {
  try {
    await main()
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : error}\n`)
    process.exitCode = 1
  }
}
