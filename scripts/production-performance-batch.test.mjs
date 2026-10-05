import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import {
  access,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import {
  measureRustFontBatch,
  validateFontBatchOutputs,
} from './production-performance-batch.mjs'

const source = Buffer.from('test production fixture')
const fixture = {
  byteLength: source.byteLength,
  sha256: createHash('sha256').update(source).digest('hex'),
  expected: {
    familyName: 'Test Font',
    glyphCount: 100,
    deliveryTables: ['fvar', 'gvar'],
  },
}
const metadata = {
  familyName: 'Test Font',
  glyphCount: 12,
  tables: ['fvar', 'gvar'],
}
const coverage = { requested: [65], supported: [65], missing: [] }

function woff2Bytes(marker = 0) {
  const contents = Buffer.alloc(64, marker)
  contents.write('wOF2')
  contents.writeUInt32BE(contents.byteLength, 8)
  return contents
}

test('validates every batch output and rejects missing, corrupt, or unmodified fonts', async () => {
  const outputDir = await mkdtemp(join(tmpdir(), 'fontmin-batch-outputs-'))
  const outputPath = join(outputDir, 'input-00000.woff2')
  let inspectedMetadata = metadata
  let inspectedCoverage = coverage
  const validate = () =>
    validateFontBatchOutputs({
      executable: 'unused',
      fixture,
      inputCount: 1,
      outputDir,
      inspect: async () => ({
        metadata: inspectedMetadata,
        coverage: inspectedCoverage,
      }),
    })

  try {
    await assert.rejects(validate(), /exactly one WOFF2/u)
    await writeFile(outputPath, woff2Bytes())
    const output = await validate()
    assert.equal(output.outputBytes, 64)
    assert.match(output.outputSha256, /^[a-f\d]{64}$/u)

    await writeFile(outputPath, woff2Bytes(1))
    const changedOutput = await validate()
    assert.notEqual(changedOutput.outputSha256, output.outputSha256)
    await writeFile(outputPath, woff2Bytes().subarray(0, 48))
    await assert.rejects(validate(), /complete WOFF2/u)
    await writeFile(outputPath, woff2Bytes())
    inspectedMetadata = { ...metadata, glyphCount: 100 }
    await assert.rejects(validate(), /non-empty subset/u)
    inspectedMetadata = { ...metadata, glyphCount: 0 }
    await assert.rejects(validate(), /non-empty subset/u)
    inspectedMetadata = { ...metadata, tables: ['fvar'] }
    await assert.rejects(validate(), /retain gvar/u)
    inspectedMetadata = metadata
    inspectedCoverage = { ...coverage, supported: [], missing: [65] }
    await assert.rejects(validate(), /cover every requested character/u)
    inspectedCoverage = coverage
    await writeFile(join(outputDir, 'extra.woff2'), woff2Bytes())
    await assert.rejects(validate(), /exactly one WOFF2/u)
  } finally {
    await rm(outputDir, { force: true, recursive: true })
  }
})

async function measureFakeBatch(
  cacheState,
  { corruptWarmOutput = false, rewriteWarmCache = false } = {},
) {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-batch-test-'))
  const fixturePath = join(root, 'source.ttf')
  let runs = 0
  let workDir

  try {
    await writeFile(fixturePath, source)
    const measurement = await measureRustFontBatch(
      { inputCount: 2, threads: 4, cacheState },
      {
        executable: 'test-cli',
        fixture,
        fixturePath,
        inspect: async () => ({ metadata, coverage }),
        runProcess: async (executable, arguments_) => {
          assert.equal(executable, 'test-cli')
          assert.deepEqual(arguments_.slice(0, 2), ['build', '--config'])
          const config = JSON.parse(await readFile(arguments_[2], 'utf8'))
          workDir = config.cwd
          runs += 1
          assert.equal(config.parallel.threads.count, 4)
          assert.equal(config.cache.enabled, true)
          assert.deepEqual(config.outputs, [{ format: 'woff2', clone: false }])
          assert.ok(config.subset.text.includes('世界'))
          assert.equal(config.subset.missingGlyphs, 'error')
          const inputFiles = await readdir(join(workDir, 'inputs'))
          assert.equal(inputFiles.length, 2)
          assert.deepEqual(
            await readFile(join(workDir, 'inputs/input-00000.ttf')),
            source,
          )

          // Both the cold build and measured warm build must start without stale output files.
          await assert.rejects(access(join(workDir, 'output')), {
            code: 'ENOENT',
          })
          await mkdir(join(workDir, 'output'))
          for (let index = 0; index < 2; index += 1) {
            await writeFile(
              join(workDir, 'output', `input-0000${index}.woff2`),
              woff2Bytes(runs === 2 && corruptWarmOutput ? 1 : 0),
            )
          }
          if (runs === 1 || rewriteWarmCache) {
            await mkdir(join(workDir, 'cache/v1'), { recursive: true })
            if (runs === 2) {
              await rm(join(workDir, 'cache/v1/index.json'))
            }
            await writeFile(
              join(workDir, 'cache/v1/index.json'),
              JSON.stringify({ entries: { first: {}, second: {} } }),
            )
          }
          return { maxRssMiB: runs === 1 ? 96 : 32 }
        },
      },
    )

    assert.equal(runs, cacheState === 'warm' ? 2 : 1)
    assert.equal(measurement.maxRssMiB, cacheState === 'warm' ? 32 : 96)
    assert.equal(measurement.outputBytes, 128)
    return measurement
  } finally {
    if (workDir !== undefined) {
      await assert.rejects(access(workDir), { code: 'ENOENT' })
    }
    await rm(root, { force: true, recursive: true })
  }
}

test('isolates cold and warm cache setup and measures only the final CLI process', async () => {
  const cold = await measureFakeBatch('cold')
  const warm = await measureFakeBatch('warm')
  assert.equal(warm.outputSha256, cold.outputSha256)
})

test('fails warm measurements when restored bytes change or the cache misses', async () => {
  await assert.rejects(
    measureFakeBatch('warm', { corruptWarmOutput: true }),
    /warm cache output differs/u,
  )
  await assert.rejects(
    measureFakeBatch('warm', { rewriteWarmCache: true }),
    /rewrote the cache index/u,
  )
})
