import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { promoteFuzzArtifacts } from './promote-fuzz-artifacts.mjs'

test('promotes minimized crashes into a content-addressed permanent corpus', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-fuzz-promotion-'))
  const artifactsDirectory = join(root, 'fuzz/minimized/public_api')
  const regressionsDirectory = join(root, 'fuzz/regressions/public_api')
  const contents = Buffer.from([7, 0x4f, 0x54, 0x54, 0x4f])
  const sha256 = createHash('sha256').update(contents).digest('hex')

  try {
    await Promise.all([
      mkdir(artifactsDirectory, { recursive: true }),
      mkdir(regressionsDirectory, { recursive: true }),
    ])
    await writeFile(join(artifactsDirectory, 'crash-input'), contents)
    await writeFile(
      join(regressionsDirectory, 'manifest.json'),
      `${JSON.stringify({ cases: [], schemaVersion: 1 }, null, 2)}\n`,
    )

    const result = await promoteFuzzArtifacts({
      artifactsDirectory,
      commit: 'a'.repeat(40),
      regressionsDirectory,
      root,
      runUrl: 'https://github.com/fontmin-rs/fontmin-rs/actions/runs/1',
    })

    assert.equal(result.promoted.length, 1)
    assert.deepEqual(result.promoted[0]?.operation, {
      id: 7,
      name: 'otfToTtf',
    })
    assert.equal(result.promoted[0]?.target, 'public_api')

    const path = join(regressionsDirectory, `crash-${sha256}.bin`)
    assert.deepEqual(await readFile(path), contents)
    assert.equal(
      await readFile(`${path}.sha256`, 'utf8'),
      `${sha256}  fuzz/regressions/public_api/crash-${sha256}.bin\n`,
    )

    const second = await promoteFuzzArtifacts({
      artifactsDirectory,
      commit: 'a'.repeat(40),
      regressionsDirectory,
      root,
      runUrl: 'https://github.com/fontmin-rs/fontmin-rs/actions/runs/1',
    })

    assert.equal(second.promoted.length, 0)
    assert.equal(second.total, 1)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

test('records the focused target operation when promoting a crash', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-focused-promotion-'))
  const artifactsDirectory = join(root, 'fuzz/minimized/output_naming')
  const regressionsDirectory = join(root, 'fuzz/regressions/output_naming')
  const contents = Buffer.from([1, ...Buffer.from('../woff2')])

  try {
    await mkdir(artifactsDirectory, { recursive: true })
    await writeFile(join(artifactsDirectory, 'crash-input'), contents)

    const result = await promoteFuzzArtifacts({
      artifactsDirectory,
      commit: 'b'.repeat(40),
      regressionsDirectory,
      root,
      runUrl: 'https://github.com/fontmin-rs/fontmin-rs/actions/runs/2',
      target: 'output_naming',
    })

    assert.deepEqual(result.promoted[0]?.operation, {
      id: 1,
      name: 'renameExtension',
    })
    assert.equal(result.promoted[0]?.target, 'output_naming')
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})
