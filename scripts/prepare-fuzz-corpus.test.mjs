import assert from 'node:assert/strict'
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { prepareFuzzCorpus } from './prepare-fuzz-corpus.mjs'

test('expands canonical fixtures across public fuzz operations', async () => {
  const outputDirectory = await mkdtemp(join(tmpdir(), 'fontmin-fuzz-corpus-'))

  try {
    const result = await prepareFuzzCorpus({ outputDirectory })
    const entries = await readdir(outputDirectory)

    assert.equal(result.count, 91)
    assert.equal(entries.length, 91)
    assert.ok(entries.includes('seed-0-malformed-not-a-font.bin'))
    assert.ok(
      entries.includes(
        'seed-7-malformed-cff-index-offset-outside-data.otf.hex',
      ),
    )
    assert.ok(entries.includes('seed-7-valid-font-awesome-free-solid-900.otf'))
    assert.ok(
      entries.includes('seed-2-malformed-subset-table-count-overflow.ttf.hex'),
    )

    const inspectSeed = await readFile(
      join(outputDirectory, 'seed-0-malformed-not-a-font.bin'),
    )

    assert.equal(inspectSeed[0], 0)
    assert.equal(inspectSeed.subarray(1).toString('utf8'), 'not-a-font\n')

    const cffSeed = await readFile(
      join(
        outputDirectory,
        'seed-7-malformed-cff-index-offset-outside-data.otf.hex',
      ),
    )

    assert.equal(cffSeed[0], 7)
    assert.equal(cffSeed.subarray(1, 5).toString('ascii'), 'OTTO')
    assert.equal(cffSeed.length, 239)
  } finally {
    await rm(outputDirectory, { force: true, recursive: true })
  }
})
