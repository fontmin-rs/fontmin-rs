import { execFile, spawn } from 'node:child_process'
import { dirname, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'
import { fuzzTargetNames } from './fuzz-operations.mjs'

const workspaceRoot = dirname(import.meta.dirname)
const executeFile = promisify(execFile)

async function runTarget(target, rustc) {
  await new Promise((resolvePromise, reject) => {
    const child = spawn(
      'cargo',
      [
        'fuzz',
        'run',
        target,
        '--sanitizer',
        'address',
        '--',
        '-runs=256',
        '-max_len=1048576',
        '-timeout=10',
      ],
      {
        cwd: workspaceRoot,
        env: {
          ...process.env,
          RUSTC: rustc,
        },
        stdio: 'inherit',
      },
    )

    child.once('error', reject)
    child.once('exit', (code, signal) => {
      if (code === 0) {
        resolvePromise()
      } else {
        reject(
          new Error(
            `${target} fuzz smoke failed with ${signal ?? `exit code ${code}`}`,
          ),
        )
      }
    })
  })
}

export async function runFuzzSmoke() {
  let rustc = process.env.RUSTC

  if (rustc === undefined) {
    const result = await executeFile('rustup', [
      'which',
      '--toolchain',
      'nightly',
      'rustc',
    ])

    rustc = result.stdout.trim()
  }

  for (const target of fuzzTargetNames) {
    await runTarget(target, rustc)
  }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  await runFuzzSmoke()
}
