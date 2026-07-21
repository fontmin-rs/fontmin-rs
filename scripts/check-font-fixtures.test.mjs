import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { checkFontFixtures } from './check-font-fixtures.mjs'

test('verifies the repository font fixture inventory', async () => {
  const result = await checkFontFixtures()

  assert.equal(result.count, 3)
  assert.deepEqual(result.paths, [
    'fixtures/fonts/otf/source-sans-3-regular.otf',
    'fixtures/fonts/otf/source-serif-4-variable-roman.otf',
    'fixtures/fonts/ttf/roboto-regular.ttf',
  ])
})

test('rejects a companion checksum that differs from the manifest', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-fixtures-'))
  const fontDirectory = join(root, 'fixtures/fonts/ttf')
  const fontPath = join(fontDirectory, 'test.ttf')
  const contents = Buffer.concat([
    Buffer.from([0, 1, 0, 0]),
    Buffer.from('glyf'),
  ])
  const sha256 = createHash('sha256').update(contents).digest('hex')

  try {
    await mkdir(fontDirectory, { recursive: true })
    await writeFile(fontPath, contents)
    await writeFile(
      `${fontPath}.sha256`,
      `${'0'.repeat(64)}  fixtures/fonts/ttf/test.ttf\n`,
    )
    await writeFile(
      join(root, 'fixtures/fonts/manifest.json'),
      JSON.stringify({
        fonts: [
          {
            container: 'ttf',
            coverage: ['test'],
            outlines: 'glyf',
            path: 'fixtures/fonts/ttf/test.ttf',
            sha256,
            source: {
              license: 'MIT',
              licenseUrl: 'https://example.com/license',
              project: 'test',
              url: 'https://example.com/test.ttf',
            },
            variation: 'static',
          },
        ],
        schemaVersion: 1,
      }),
    )
    await assert.rejects(
      checkFontFixtures({ root }),
      /test\.ttf\.sha256 does not match the manifest/u,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})
