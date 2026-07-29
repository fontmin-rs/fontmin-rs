import { copyFile, mkdir, readdir } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { readNativeReleaseLayout } from './native-release-layout.mjs'

const artifactPattern = /^fontmin_rs\.(?<platform>.+)\.node$/u

export async function copyNativeArtifacts({ npmDir, outputDir, root }) {
  const { entries } = await readNativeReleaseLayout({ root })
  const packageDirectoryByPlatform = new Map(
    entries.map(entry => [entry.platform, entry.packageDirectory]),
  )
  const outputEntries = await readdir(outputDir, { withFileTypes: true })
  const copied = []

  for (const outputEntry of outputEntries) {
    if (!outputEntry.isFile()) {
      continue
    }

    const match = artifactPattern.exec(outputEntry.name)
    if (!match?.groups?.platform) {
      continue
    }

    const packageDirectory = packageDirectoryByPlatform.get(
      match.groups.platform,
    )
    if (!packageDirectory) {
      throw new Error(
        `No platform package mapping exists for native artifact ${outputEntry.name}`,
      )
    }

    const destination = join(npmDir, packageDirectory, outputEntry.name)
    await mkdir(dirname(destination), { recursive: true })
    await copyFile(join(outputDir, outputEntry.name), destination)
    copied.push(`${packageDirectory}/${outputEntry.name}`)
  }

  if (copied.length === 0) {
    throw new Error(`No fontmin_rs native artifacts found in ${outputDir}`)
  }

  return copied.sort()
}

const entrypoint = process.argv[1] && resolve(process.argv[1])
if (entrypoint === import.meta.filename) {
  const workspaceRoot = dirname(import.meta.dirname)
  const copied = await copyNativeArtifacts({
    npmDir: join(workspaceRoot, 'npm'),
    outputDir: join(workspaceRoot, 'napi', 'fontmin', 'src-js'),
  })

  console.log(`Copied native artifacts: ${copied.join(', ')}`)
}
