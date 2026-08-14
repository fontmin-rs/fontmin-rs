import { existsSync, mkdtempSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { resolve } from 'node:path'
import { Transform } from 'node:stream'
import Vinyl from 'vinyl'
import { expect, it } from 'vitest'
import Fontmin from '../src/vinyl'
import { fixture } from './api-fixtures'

it('runs typed plugins with real Vinyl files', async () => {
  const files = await new Fontmin()
    .src(fixture)
    .use(Fontmin.glyph({ text: 'Hello' }))
    .use(Fontmin.ttf2woff())
    .runAsync()

  expect(files.every(file => Vinyl.isVinyl(file))).toBe(true)
  expect(files.map(file => file.relative).toSorted()).toStrictEqual([
    'roboto-regular.ttf',
    'roboto-regular.woff',
  ])
})

it('composes typed plugins with Vinyl transform factories', async () => {
  const files = await new Fontmin()
    .src(fixture)
    .use(Fontmin.ttf2woff({ clone: false }))
    .use(
      () =>
        new Transform({
          objectMode: true,
          transform(file: Vinyl, _encoding, callback) {
            file.stem = 'renamed'
            callback(null, file)
          },
        }),
    )
    .runAsync()

  expect(files).toHaveLength(1)
  expect(files[0]?.relative).toBe('renamed.woff')
})

it('preserves Vinyl source and destination options and writes files', async () => {
  const outputDir = mkdtempSync(resolve(tmpdir(), 'fontmin-rs-vinyl-'))
  const sourceOptions = { base: resolve(fixture, '..') }
  const destinationOptions = { overwrite: true }
  const fontmin = new Fontmin()
    .src(fixture, sourceOptions)
    .use(Fontmin.ttf2woff({ clone: false }))
    .dest(outputDir, destinationOptions)

  try {
    expect(fontmin.src()).toStrictEqual([fixture, sourceOptions])
    expect(fontmin.dest()).toStrictEqual([outputDir, destinationOptions])

    const files = await fontmin.runAsync()

    expect(files[0]?.base).toBe(outputDir)
    expect(existsSync(resolve(outputDir, 'roboto-regular.woff'))).toBe(true)
  } finally {
    rmSync(outputDir, { recursive: true, force: true })
  }
})

it('returns a stream and emits the classic default outputs', async () => {
  const streamedFiles: Vinyl[] = []
  let returnedStream: NodeJS.ReadableStream | undefined
  const callbackFiles = await new Promise<Vinyl[]>((resolveFiles, reject) => {
    const stream = new Fontmin().src(fixture).run((error, files) => {
      if (error !== null) {
        reject(error)
        return
      }

      resolveFiles(files ?? [])
    })

    returnedStream = stream
    stream.on('data', file => streamedFiles.push(file as Vinyl))
  })

  expect(returnedStream).toBeDefined()
  expect(streamedFiles).toStrictEqual(callbackFiles)
  expect(callbackFiles.map(file => file.relative).toSorted()).toStrictEqual([
    'roboto-regular.css',
    'roboto-regular.eot',
    'roboto-regular.svg',
    'roboto-regular.ttf',
    'roboto-regular.woff',
    'roboto-regular.woff2',
  ])
})
