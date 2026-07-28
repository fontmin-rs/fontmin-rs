import assert from 'node:assert/strict'
import { mkdtemp, readFile, readdir, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { fuzzOperations } from './fuzz-operations.mjs'
import { prepareFuzzCorpus } from './prepare-fuzz-corpus.mjs'

test('expands canonical fixtures across public fuzz operations', async () => {
  const outputDirectory = await mkdtemp(join(tmpdir(), 'fontmin-fuzz-corpus-'))

  try {
    const result = await prepareFuzzCorpus({ outputDirectory })
    const entries = await readdir(outputDirectory)
    const [malformedManifest, regressionManifest] = await Promise.all([
      readFile(
        new URL('../fixtures/malformed/manifest.json', import.meta.url),
        'utf8',
      ).then(JSON.parse),
      readFile(
        new URL(
          '../fuzz/regressions/public_api/manifest.json',
          import.meta.url,
        ),
        'utf8',
      ).then(JSON.parse),
    ])
    const expectedMalformedCount =
      malformedManifest.cases.length * fuzzOperations.length
    const expectedCount =
      expectedMalformedCount +
      result.validSeedCount +
      regressionManifest.cases.length

    assert.equal(result.count, expectedCount)
    assert.equal(result.malformedSeedCount, expectedMalformedCount)
    assert.equal(result.regressionSeedCount, regressionManifest.cases.length)
    assert.equal(entries.length, expectedCount)
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
    assert.ok(
      entries.includes(
        'seed-regression-8495d0f61e827485b6d1fdbe60b5f5fea189233c77ad2afc128f24db29409d64.bin',
      ),
    )
    assert.ok(
      entries.includes(
        'seed-regression-70e741943b74b3527558216a88b395d4715dbb868bcd6b6e1c7cb7339edefe06.bin',
      ),
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
