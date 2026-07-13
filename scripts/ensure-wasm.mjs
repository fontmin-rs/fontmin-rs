import { execFile } from 'node:child_process'
import { access } from 'node:fs/promises'
import { homedir } from 'node:os'
import { delimiter, dirname, join, resolve } from 'node:path'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)
const wasmArtifacts = [
  'fontmin_wasm_core.js',
  'fontmin_wasm_core.d.ts',
  'fontmin_wasm_core_bg.wasm',
].map(file => join(workspaceRoot, 'wasm', 'fontmin', 'src', 'generated', file))

export async function runPnpm(
  args,
  { execute = executeFile, platform = process.platform } = {},
) {
  const cargoBin = join(homedir(), '.cargo', 'bin')

  await execute('pnpm', args, {
    cwd: workspaceRoot,
    env: {
      ...process.env,
      PATH: [cargoBin, process.env.PATH].filter(Boolean).join(delimiter),
    },
    shell: platform === 'win32',
  })
}

export async function ensureWasm({
  artifacts = wasmArtifacts,
  buildPackage = () => runPnpm(['-C', 'wasm/fontmin', 'run', 'build:js']),
  buildWasm = () => runPnpm(['-C', 'wasm/fontmin', 'run', 'build:wasm']),
} = {}) {
  const available = await Promise.all(
    artifacts.map(async artifact => {
      try {
        await access(artifact)
        return true
      } catch {
        return false
      }
    }),
  )
  const generated = !available.every(Boolean)

  if (generated) {
    await buildWasm()
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
