import { execFile } from 'node:child_process'
import { createHash } from 'node:crypto'
import { access, readdir, readFile, stat, writeFile } from 'node:fs/promises'
import { homedir } from 'node:os'
import { delimiter, dirname, join, relative, resolve } from 'node:path'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)
const wasmArtifacts = [
  'fontmin_wasm_core.js',
  'fontmin_wasm_core.d.ts',
  'fontmin_wasm_core_bg.wasm',
].map(file => join(workspaceRoot, 'wasm', 'fontmin', 'src', 'generated', file))
const wasmSourceStamp = join(
  workspaceRoot,
  'wasm',
  'fontmin',
  'src',
  'generated',
  'fontmin_wasm_core.source.sha256',
)
const wasmSourceRoots = [
  'Cargo.lock',
  'Cargo.toml',
  'rust-toolchain.toml',
  'crates',
  'vendor',
  'wasm/fontmin-core',
].map(path => join(workspaceRoot, path))

async function collectFiles(path) {
  const metadata = await stat(path)

  if (metadata.isFile()) {
    return [path]
  }

  const entries = await readdir(path, { withFileTypes: true })
  const files = await Promise.all(
    entries.map(entry => collectFiles(join(path, entry.name))),
  )

  return files.flat()
}

async function sourceDigest(sourceRoots) {
  const nestedFiles = await Promise.all(
    sourceRoots.map(root => collectFiles(root)),
  )
  const files = nestedFiles.flat().toSorted()
  const hash = createHash('sha256')

  for (const file of files) {
    hash.update(relative(workspaceRoot, file))
    hash.update('\0')
    hash.update(await readFile(file))
    hash.update('\0')
  }

  return hash.digest('hex')
}

export async function runPnpm(
  args,
  { execute = executeFile, platform = process.platform } = {},
) {
  const cargoBin = join(homedir(), '.cargo', 'bin')

  await execute('pnpm', args, {
    cwd: workspaceRoot,
    env: {
      ...process.env,
      PATH: [process.env.PATH, cargoBin].filter(Boolean).join(delimiter),
    },
    shell: platform === 'win32',
  })
}

export async function ensureWasm({
  artifacts = wasmArtifacts,
  buildPackage = () => runPnpm(['-C', 'wasm/fontmin', 'run', 'build:js']),
  buildWasm = () => runPnpm(['-C', 'wasm/fontmin', 'run', 'build:wasm']),
  sourceRoots = wasmSourceRoots,
  sourceStamp = wasmSourceStamp,
} = {}) {
  const [digest, available] = await Promise.all([
    sourceDigest(sourceRoots),
    Promise.all(
      artifacts.map(async artifact => {
        try {
          await access(artifact)
          return true
        } catch {
          return false
        }
      }),
    ),
  ])
  let previousDigest

  try {
    const stamp = await readFile(sourceStamp, 'utf8')

    previousDigest = stamp.trim()
  } catch (error) {
    if (error?.code !== 'ENOENT') {
      throw error
    }
  }

  const generated = !available.every(Boolean) || previousDigest !== digest

  if (generated) {
    await buildWasm()
    await writeFile(sourceStamp, `${digest}\n`)
  }
  await buildPackage()

  return generated
}

const entrypoint = process.argv[1] && resolve(process.argv[1])
if (entrypoint === import.meta.filename) {
  const generated = await ensureWasm()
  console.log(
    generated
      ? 'Generated and built WASM package.'
      : 'Reused generated artifacts and built WASM package.',
  )
}
