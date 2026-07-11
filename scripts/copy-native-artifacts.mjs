import { copyFile, mkdir, readdir } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const packageDirectoryByPlatform = new Map([
  ['darwin-arm64', 'binding-darwin-arm64'],
  ['darwin-x64', 'binding-darwin-x64'],
  ['linux-arm64-gnu', 'binding-linux-arm64-gnu'],
  ['linux-arm64-musl', 'binding-linux-arm64-musl'],
  ['linux-x64-gnu', 'binding-linux-x64-gnu'],
  ['linux-x64-musl', 'binding-linux-x64-musl'],
  ['win32-arm64-msvc', 'binding-win32-arm64-msvc'],
  ['win32-x64-msvc', 'binding-win32-x64-msvc'],
])

const artifactPattern = /^fontmin_rs\.(?<platform>.+)\.node$/u

export async function copyNativeArtifacts({ npmDir, outputDir }) {
  const entries = await readdir(outputDir, { withFileTypes: true })
  const copied = []

  for (const entry of entries) {
    if (!entry.isFile()) continue

    const match = artifactPattern.exec(entry.name)
    if (!match?.groups?.platform) continue

    const packageDirectory = packageDirectoryByPlatform.get(
      match.groups.platform,
    )
    if (!packageDirectory) {
      throw new Error(
        `No platform package mapping exists for native artifact ${entry.name}`,
      )
    }

    const destination = join(npmDir, packageDirectory, entry.name)
    await mkdir(dirname(destination), { recursive: true })
    await copyFile(join(outputDir, entry.name), destination)
    copied.push(`${packageDirectory}/${entry.name}`)
  }

  if (copied.length === 0) {
    throw new Error(`No fontmin_rs native artifacts found in ${outputDir}`)
  }

  return copied.sort()
}

const entrypoint = process.argv[1] && resolve(process.argv[1])
if (entrypoint === fileURLToPath(import.meta.url)) {
  const workspaceRoot = dirname(dirname(fileURLToPath(import.meta.url)))
  const copied = await copyNativeArtifacts({
    npmDir: join(workspaceRoot, 'npm'),
    outputDir: join(workspaceRoot, 'napi', 'fontmin', 'src-js'),
  })

  console.log(`Copied native artifacts: ${copied.join(', ')}`)
}
