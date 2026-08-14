import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { expect, it } from 'vitest'
import {
  discoverWebText,
  extractWebText,
  optimize,
  subsetTtf,
} from '../src/index'

const fontFixture = new URL(
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
  import.meta.url,
)

it('discovers static text across common web source formats', async () => {
  const cwd = await mkdtemp(join(tmpdir(), 'fontmin-web-text-'))

  try {
    await Promise.all([
      writeFile(
        join(cwd, 'index.html'),
        '<main title="提示">Hello &amp; 世界</main><script>const label = "保存"</script>',
      ),
      writeFile(
        join(cwd, 'component.tsx'),
        'export const View = () => <button>{"提交"}</button>',
      ),
      writeFile(join(cwd, 'theme.css'), '.icon::before { content: "→"; }'),
      writeFile(
        join(cwd, 'messages.json'),
        JSON.stringify({ button: '取消', nested: ['完成'] }),
      ),
    ])

    const result = await discoverWebText({
      cwd,
      files: ['**/*.{css,html,json,tsx}'],
    })

    expect(result.files).toStrictEqual([
      'component.tsx',
      'index.html',
      'messages.json',
      'theme.css',
    ])
    for (const expected of [
      'Hello',
      '世界',
      '提示',
      '保存',
      '提交',
      '取消',
      '完成',
      '→',
    ]) {
      for (const character of expected) {
        expect(result.text).toContain(character)
      }
    }
    expect(result.unicodes).toStrictEqual(
      [...result.text].map(character => character.codePointAt(0)),
    )
  } finally {
    await rm(cwd, { recursive: true, force: true })
  }
})

it('decodes markup entities, script escapes, and CSS content escapes', () => {
  expect(extractWebText('<p>A&amp;B</p>', '.html')).toContain('A&B')
  expect(extractWebText(String.raw`const value = "\u4E2D"`, '.ts')).toContain(
    '中',
  )
  expect(
    extractWebText(String.raw`.icon { content: "\2192" }`, '.css'),
  ).toContain('→')
})

it('rejects empty requests and unmatched globs', async () => {
  await expect(discoverWebText({ files: [] })).rejects.toThrow(
    'requires at least one file or glob',
  )
  await expect(
    discoverWebText({ files: ['does-not-exist/**/*.html'] }),
  ).rejects.toThrow('content glob matched no files')
})

it('feeds discovered content globs into top-level subset configuration', async () => {
  const cwd = await mkdtemp(join(tmpdir(), 'fontmin-web-content-'))

  try {
    await writeFile(join(cwd, 'page.html'), '<h1 title="Welcome">Hello</h1>')
    const input = await readFile(fontFixture)
    const discovered = await discoverWebText({ cwd, files: ['*.html'] })
    const assets = await optimize({
      cwd,
      input: [input],
      outputs: ['ttf'],
      subset: { content: ['*.html'] },
    })

    expect(assets).toHaveLength(1)
    expect(assets[0]!.contents).toStrictEqual(
      subsetTtf(input, { text: discovered.text }),
    )
  } finally {
    await rm(cwd, { recursive: true, force: true })
  }
})
