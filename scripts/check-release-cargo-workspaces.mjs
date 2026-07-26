import { execFileSync } from 'node:child_process'
import { pathToFileURL } from 'node:url'

const cargoCheckArguments = [
  ['check', '--workspace'],
  ['check', '--manifest-path', 'fuzz/Cargo.toml'],
]

export function checkReleaseCargoWorkspaces(execute = execFileSync) {
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
