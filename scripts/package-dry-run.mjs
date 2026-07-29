import { execFile } from 'node:child_process'
import { access, readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { promisify } from 'node:util'
import { validateNativeReleaseLayout } from './native-release-layout.mjs'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)
const primaryPackageDirectories = [
  'packages/fontmin',
  'wasm/fontmin',
  'napi/fontmin',
]

async function hasPackagedFiles(directory) {
  const packageRoot = join(workspaceRoot, directory)
  const manifest = JSON.parse(
    await readFile(join(packageRoot, 'package.json'), 'utf8'),
  )

  try {
    await Promise.all(
      manifest.files.map(file => access(join(packageRoot, file))),
    )
    return true
  } catch {
    return false
  }
}

const availablePlatformDirectories = []
const nativeReleaseEntries = await validateNativeReleaseLayout()
for (const directory of nativeReleaseEntries.map(entry => entry.directory)) {
  if (await hasPackagedFiles(directory)) {
    availablePlatformDirectories.push(directory)
  }
}

if (availablePlatformDirectories.length === 0) {
  throw new Error(
    'No platform package is ready; build and place a native artifact first.',
  )
}

const packageDirectories = [
  ...primaryPackageDirectories,
  ...availablePlatformDirectories,
]

for (const directory of packageDirectories) {
  const { stderr, stdout } = await executeFile('npm', ['pack', '--dry-run'], {
    cwd: join(workspaceRoot, directory),
  })
  process.stdout.write(stdout)
  process.stderr.write(stderr)
}

process.stdout.write(
  `Dry-run packed ${packageDirectories.length} packages (${availablePlatformDirectories.length} platform packages).\n`,
)
