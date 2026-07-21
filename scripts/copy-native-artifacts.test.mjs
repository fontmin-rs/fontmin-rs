import assert from 'node:assert/strict'
import { mkdtemp, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { copyNativeArtifacts } from './copy-native-artifacts.mjs'

test('copies native artifacts into the matching binding package', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-native-artifacts-'))
  const outputDir = join(root, 'src-js')
  const npmDir = join(root, 'npm')

  try {
    await mkdir(outputDir, { recursive: true })
    await writeFile(
      join(outputDir, 'fontmin_rs.darwin-arm64.node'),
      'native-artifact',
    )

    const copied = await copyNativeArtifacts({ npmDir, outputDir })

    assert.deepEqual(copied, [
      'binding-darwin-arm64/fontmin_rs.darwin-arm64.node',
    ])
    assert.equal(
      await readFile(
        join(npmDir, 'binding-darwin-arm64', 'fontmin_rs.darwin-arm64.node'),
        'utf8',
      ),
      'native-artifact',
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('rejects an artifact with no published package mapping', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-native-artifacts-'))
  const outputDir = join(root, 'src-js')

  try {
    await mkdir(outputDir, { recursive: true })
    await writeFile(
      join(outputDir, 'fontmin_rs.android-arm64.node'),
      'artifact',
    )

    await assert.rejects(
      copyNativeArtifacts({ npmDir: join(root, 'npm'), outputDir }),
      /no platform package/iu,
    )
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})
