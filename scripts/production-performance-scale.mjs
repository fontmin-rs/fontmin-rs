import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import {
  access,
  link,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { performance } from 'node:perf_hooks'

const source = Buffer.from([0])

function createScaleFileNames(inputCount) {
  return Array.from(
    { length: inputCount },
    (_, index) => `input-${String(index).padStart(5, '0')}.bin`,
  )
}

async function createScaleInputs(root, inputCount) {
  const inputs = join(root, 'inputs')
  const seed = join(root, 'seed.bin')
  const fileNames = createScaleFileNames(inputCount)

  await mkdir(inputs)
  await writeFile(seed, source)
  for (let start = 0; start < inputCount; start += 256) {
    await Promise.all(
      fileNames
        .slice(start, start + 256)
        .map(fileName => link(seed, join(inputs, fileName))),
    )
  }
}

export async function validateBuildScaleOutputs({ inputCount, outputDir }) {
  const fileNames = createScaleFileNames(inputCount)
  const actualFileNames = await readdir(outputDir)
  assert.deepEqual(
    actualFileNames.toSorted(),
    fileNames.toSorted(),
    'build-scale must produce exactly one unchanged file per input',
  )

  const hash = createHash('sha256')
  let outputBytes = 0
  for (const fileName of fileNames) {
    const contents = await readFile(join(outputDir, fileName))
    assert.deepEqual(
      contents,
      source,
      `${fileName} must retain the input bytes`,
    )
    hash.update(fileName).update('\0').update(contents)
    outputBytes += contents.byteLength
  }

  return { outputBytes, outputSha256: hash.digest('hex') }
}

export async function measureRustBuildScale(stage, { executable, runProcess }) {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-build-scale-'))
  const inputCount = stage.inputCount ?? 10_000
  const threads = stage.threads ?? 4
  const outputDir = join(root, 'output')
  const configPath = join(root, 'fontmin.config.json')

  try {
    await access(executable)
    await createScaleInputs(root, inputCount)
    await writeFile(
      configPath,
      `${JSON.stringify({
        cache: { dir: 'cache', enabled: false },
        css: null,
        cwd: root,
        diagnostics: { level: 'silent' },
        input: ['inputs/*.bin'],
        outDir: 'output',
        outputs: [],
        parallel: { perFile: true, threads: { count: threads } },
        plugins: [],
      })}\n`,
    )

    const startedAt = performance.now()
    const { maxRssMiB } = await runProcess(
      executable,
      ['build', '--config', configPath],
      { cwd: root },
    )
    const latencyMs = performance.now() - startedAt
    const output = await validateBuildScaleOutputs({ inputCount, outputDir })

    return { latencyMs, maxRssMiB, ...output }
  } finally {
    await rm(root, { force: true, recursive: true })
  }
}
