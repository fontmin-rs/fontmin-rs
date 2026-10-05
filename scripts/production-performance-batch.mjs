import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { createHash } from 'node:crypto'
import {
  link,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  stat,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { performance } from 'node:perf_hooks'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const subsetText = 'Hello, 世界！中文字体性能测试。'

async function inspectOutput(executable, outputPath) {
  const [inspection, coverage] = await Promise.all([
    executeFile(executable, ['inspect', outputPath, '--json']),
    executeFile(executable, [
      'coverage',
      outputPath,
      '--text',
      subsetText,
      '--json',
    ]),
  ])

  return {
    ...JSON.parse(inspection.stdout),
    coverage: JSON.parse(coverage.stdout),
  }
}

export async function validateFontBatchOutputs({
  executable,
  fixture,
  inputCount,
  inspect = inspectOutput,
  outputDir,
}) {
  const fileNames = Array.from(
    { length: inputCount },
    (_, index) => `input-${String(index).padStart(5, '0')}.woff2`,
  )
  const actualFileNames = await readdir(outputDir)
  assert.deepEqual(
    actualFileNames.toSorted(),
    fileNames,
    'font batch must produce exactly one WOFF2 file per input',
  )

  const hash = createHash('sha256')
  let outputBytes = 0
  for (const fileName of fileNames) {
    const outputPath = join(outputDir, fileName)
    const contents = await readFile(outputPath)

    assert.ok(
      contents.byteLength >= 48 &&
        contents.toString('ascii', 0, 4) === 'wOF2' &&
        contents.readUInt32BE(8) === contents.byteLength,
      `${fileName} must contain a complete WOFF2 font`,
    )
    const { metadata, coverage } = await inspect(executable, outputPath)

    assert.equal(metadata.familyName, fixture.expected.familyName)
    assert.ok(
      metadata.glyphCount > 1 &&
        metadata.glyphCount < fixture.expected.glyphCount,
      `${fileName} must contain a non-empty subset`,
    )
    for (const table of fixture.expected.deliveryTables ?? []) {
      assert.ok(
        metadata.tables.includes(table),
        `${fileName} must retain ${table}`,
      )
    }
    assert.ok(
      coverage.requested.length > 0 &&
        coverage.missing.length === 0 &&
        coverage.supported.length === coverage.requested.length,
      `${fileName} must cover every requested character`,
    )

    hash.update(fileName).update('\0').update(contents)
    outputBytes += contents.byteLength
  }

  return { outputBytes, outputSha256: hash.digest('hex') }
}

export async function measureRustFontBatch(
  stage,
  { executable, fixture, fixturePath, inspect = inspectOutput, runProcess },
) {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-font-batch-'))
  const outputDir = join(root, 'output')
  const configPath = join(root, 'fontmin.config.json')
  const cacheIndexPath = join(root, 'cache/v1/index.json')

  try {
    const contents = await readFile(fixturePath)
    assert.equal(
      contents.byteLength,
      fixture.byteLength,
      'font batch fixture length changed',
    )
    assert.equal(
      createHash('sha256').update(contents).digest('hex'),
      fixture.sha256,
      'font batch fixture checksum changed',
    )
    await mkdir(join(root, 'inputs'))
    // Keep hard links on the temporary filesystem, even if the fixture is on another volume.
    await writeFile(join(root, 'seed.ttf'), contents)
    for (let index = 0; index < stage.inputCount; index += 1) {
      await link(
        join(root, 'seed.ttf'),
        join(root, 'inputs', `input-${String(index).padStart(5, '0')}.ttf`),
      )
    }
    await writeFile(
      configPath,
      `${JSON.stringify({
        cache: { dir: 'cache', enabled: true },
        css: null,
        cwd: root,
        diagnostics: { level: 'silent' },
        input: ['inputs/*.ttf'],
        outDir: 'output',
        outputs: [{ format: 'woff2', clone: false }],
        parallel: { perFile: true, threads: { count: stage.threads } },
        preserveOriginal: false,
        subset: { text: subsetText, missingGlyphs: 'error' },
      })}\n`,
    )

    const run = () =>
      runProcess(executable, ['build', '--config', configPath], { cwd: root })
    const validate = () =>
      validateFontBatchOutputs({
        executable,
        fixture,
        inputCount: stage.inputCount,
        inspect,
        outputDir,
      })
    let primedOutput
    let primedIndex
    if (stage.cacheState === 'warm') {
      await run()
      primedOutput = await validate()
      primedIndex = await stat(cacheIndexPath, { bigint: true })
      // Require the measured process to restore outputs rather than reuse files on disk.
      await rm(outputDir, { recursive: true })
    }

    const startedAt = performance.now()
    const { maxRssMiB } = await run()
    const latencyMs = performance.now() - startedAt
    const output = await validate()
    const cacheIndex = JSON.parse(await readFile(cacheIndexPath, 'utf8'))

    assert.equal(
      Object.keys(cacheIndex.entries).length,
      stage.inputCount,
      'font batch cache must contain every input',
    )
    if (primedOutput !== undefined) {
      assert.deepEqual(
        output,
        primedOutput,
        'warm cache output differs from the cold build',
      )
      const measuredIndex = await stat(cacheIndexPath, { bigint: true })
      assert.ok(
        measuredIndex.ino === primedIndex.ino &&
          measuredIndex.mtimeNs === primedIndex.mtimeNs,
        'warm font batch rewrote the cache index instead of restoring every input',
      )
    }

    return { latencyMs, maxRssMiB, ...output }
  } finally {
    await rm(root, { force: true, recursive: true })
  }
}
