import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import {
  access,
  copyFile,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rename,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import {
  measureRustBuildScale,
  validateBuildScaleOutputs,
} from './production-performance-scale.mjs'

test('validates every scale output and hashes names and bytes in deterministic order', async () => {
  const outputDir = await mkdtemp(join(tmpdir(), 'fontmin-scale-outputs-'))
  const firstOutput = join(outputDir, 'input-00000.bin')
  const secondOutput = join(outputDir, 'input-00001.bin')
  const validate = () => validateBuildScaleOutputs({ inputCount: 2, outputDir })

  try {
    await assert.rejects(validate(), /exactly one unchanged file/u)
    // Creation order must not affect the digest used to compare thread counts.
    await writeFile(secondOutput, Buffer.from([0]))
    await writeFile(firstOutput, Buffer.from([0]))
    const output = await validate()
    assert.deepEqual(output, {
      outputBytes: 2,
      outputSha256: createHash('sha256')
        .update('input-00000.bin\0\0input-00001.bin\0\0')
        .digest('hex'),
    })

    await rm(secondOutput)
    await assert.rejects(validate(), /exactly one unchanged file/u)
    for (const contents of [
      Buffer.alloc(0),
      Buffer.from([1]),
      Buffer.from([0, 0]),
    ]) {
      await writeFile(secondOutput, contents)
      await assert.rejects(
        validate(),
        /input-00001.bin must retain the input bytes/u,
      )
    }
    await writeFile(secondOutput, Buffer.from([0]))

    const unexpectedOutput = join(outputDir, 'unexpected.bin')
    await rename(secondOutput, unexpectedOutput)
    await assert.rejects(validate(), /exactly one unchanged file/u)
    await writeFile(secondOutput, Buffer.from([0]))
    await assert.rejects(validate(), /exactly one unchanged file/u)
    await rm(unexpectedOutput)
    assert.deepEqual(await validate(), output)
  } finally {
    await rm(outputDir, { force: true, recursive: true })
  }
})

async function measureFakeScale(threads, { corruptOutput = false } = {}) {
  let runs = 0
  let workDir

  try {
    const measurement = await measureRustBuildScale(
      { inputCount: 257, threads },
      {
        executable: process.execPath,
        runProcess: async (executable, arguments_, options) => {
          runs += 1
          assert.equal(executable, process.execPath)
          assert.deepEqual(arguments_.slice(0, 2), ['build', '--config'])
          const config = JSON.parse(await readFile(arguments_[2], 'utf8'))
          workDir = config.cwd
          assert.equal(options.cwd, workDir)
          assert.equal(config.parallel.threads.count, threads ?? 4)
          assert.equal(config.cache.enabled, false)
          assert.deepEqual(config.outputs, [])
          assert.deepEqual(config.plugins, [])
          assert.deepEqual(config.input, ['inputs/*.bin'])
          const inputDir = join(workDir, 'inputs')
          const inputFiles = await readdir(inputDir)
          assert.equal(inputFiles.length, 257)
          assert.equal(inputFiles.toSorted()[0], 'input-00000.bin')
          assert.equal(inputFiles.toSorted().at(-1), 'input-00256.bin')

          const outputDir = join(workDir, config.outDir)
          await assert.rejects(access(outputDir), { code: 'ENOENT' })
          await mkdir(outputDir)
          for (const fileName of inputFiles) {
            const inputPath = join(inputDir, fileName)
            assert.deepEqual(await readFile(inputPath), Buffer.from([0]))
            await copyFile(inputPath, join(outputDir, fileName))
          }
          if (corruptOutput) {
            await writeFile(
              join(outputDir, 'input-00256.bin'),
              Buffer.from([1]),
            )
          }
          return { maxRssMiB: 32 }
        },
      },
    )

    assert.equal(runs, 1)
    assert.equal(measurement.maxRssMiB, 32)
    assert.equal(measurement.outputBytes, 257)
    assert.ok(
      Number.isFinite(measurement.latencyMs) && measurement.latencyMs >= 0,
    )
    return measurement
  } finally {
    if (workDir !== undefined) {
      await assert.rejects(access(workDir), { code: 'ENOENT' })
    }
  }
}

test('supports consistent measurements across one, four, and eight threads', async () => {
  const measurements = []
  for (const threads of [1, 4, 8, undefined]) {
    measurements.push(await measureFakeScale(threads))
  }
  assert.equal(new Set(measurements.map(output => output.outputSha256)).size, 1)
})

test('rejects invalid measured output and cleans up the temporary workspace', async () => {
  await assert.rejects(
    measureFakeScale(4, { corruptOutput: true }),
    /input-00256.bin must retain the input bytes/u,
  )
})
