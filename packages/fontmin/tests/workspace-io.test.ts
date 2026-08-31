import {
  access,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  symlink,
  writeFile,
} from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { expect, it } from 'vitest'
import { writeAssets } from '../src/workspace-io'

it('atomically replaces outputs without leaving temporary files', async () => {
  const root = await mkdtemp(resolve(tmpdir(), 'fontmin-output-atomic-'))
  const outDir = resolve(root, 'out')
  const output = resolve(outDir, 'font.ttf')

  try {
    await mkdir(outDir)
    await writeFile(output, 'old')

    await writeAssets(outDir, [
      {
        contents: Buffer.from('new'),
        format: 'ttf',
        meta: {},
        path: 'font.ttf',
        sourceFormat: 'ttf',
      },
    ])

    await expect(readFile(output, 'utf8')).resolves.toBe('new')
    await expect(readdir(outDir)).resolves.toStrictEqual(['font.ttf'])
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

it.skipIf(process.platform === 'win32')(
  'rejects a symlinked output ancestor without creating outside directories',
  async () => {
    const root = await mkdtemp(resolve(tmpdir(), 'fontmin-output-symlink-'))
    const outDir = resolve(root, 'out')
    const outsideDir = resolve(root, 'outside')

    try {
      await Promise.all([mkdir(outDir), mkdir(outsideDir)])
      await symlink(outsideDir, resolve(outDir, 'linked'), 'dir')

      await expect(
        writeAssets(outDir, [
          {
            contents: Buffer.from('font'),
            format: 'ttf',
            meta: {},
            path: 'linked/nested/font.ttf',
            sourceFormat: 'ttf',
          },
        ]),
      ).rejects.toThrow('asset path resolves outside its destination directory')
      await expect(access(resolve(outsideDir, 'nested'))).rejects.toMatchObject(
        {
          code: 'ENOENT',
        },
      )
    } finally {
      await rm(root, { force: true, recursive: true })
    }
  },
)
