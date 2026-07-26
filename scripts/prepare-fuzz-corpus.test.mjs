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

    assert.equal(result.count, 64)
    assert.equal(entries.length, 64)
    assert(entries.includes('seed-0-malformed-not-a-font.bin'))
    assert(entries.includes('seed-7-valid-font-awesome-free-solid-900.otf'))

    const inspectSeed = await readFile(
      join(outputDirectory, 'seed-0-malformed-not-a-font.bin'),
    )

    assert.equal(inspectSeed[0], 0)
    assert.equal(inspectSeed.subarray(1).toString('utf8'), 'not-a-font\n')
  } finally {
    await rm(outputDirectory, { force: true, recursive: true })
  }
})
