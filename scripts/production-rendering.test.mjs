import assert from 'node:assert/strict'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import test from 'node:test'
import { deflateSync } from 'node:zlib'
import {
  assertRenderingParity,
  assertShapingParity,
  pngPixels,
  runProductionRendering,
} from './production-rendering.mjs'

test('persists setup failures and replaces a previous successful report', async () => {
  const root = await mkdtemp(join(tmpdir(), 'fontmin-rendering-failure-'))
  const directory = join(root, 'benchmarks/rendering-current')
  const path = join(directory, 'report.json')
  try {
    await mkdir(directory, { recursive: true })
    await writeFile(
      path,
      JSON.stringify({ status: 'passed', cases: ['stale'] }),
    )
    await assert.rejects(runProductionRendering({ root }), /ENOENT/u)
    let report = JSON.parse(await readFile(path, 'utf8'))
    assert.equal(report.status, 'failed')
    assert.deepEqual(report.cases, [])
    assert.match(report.error, /manifest/u)
    await mkdir(join(root, 'fixtures/production'), { recursive: true })
    await writeFile(
      join(root, 'fixtures/production/manifest.json'),
      JSON.stringify({ schemaVersion: 1, fixtures: [] }),
    )
    await assert.rejects(
      runProductionRendering({ root }),
      /Cannot find module/u,
    )
    report = JSON.parse(await readFile(path, 'utf8'))
    assert.equal(report.status, 'failed')
    assert.match(report.error, /Cannot find module/u)
  } finally {
    await rm(root, { force: true, recursive: true })
  }
})

function png({ rows = [[200, 50, 20]], filter = 0, width = 1 } = {}) {
  const header = Buffer.alloc(13)
  header.writeUInt32BE(width, 0)
  header.writeUInt32BE(rows.length, 4)
  header[8] = 8
  header[9] = 2
  const filtered = []
  for (const [y, row] of rows.entries()) {
    filtered.push(filter)
    for (const [x, value] of row.entries()) {
      const left = x < 3 ? 0 : row[x - 3]
      const up = y === 0 ? 0 : rows[y - 1][x]
      const upperLeft = y === 0 || x < 3 ? 0 : rows[y - 1][x - 3]
      const prediction = left + up - upperLeft
      const candidates = [left, up, upperLeft]
      const nearest = candidates.toSorted(
        (a, b) => Math.abs(prediction - a) - Math.abs(prediction - b),
      )[0]
      const predictors = [0, left, up, Math.floor((left + up) / 2), nearest]
      filtered.push((value - predictors[filter] + 256) % 256)
    }
  }
  const chunks = [
    ['IHDR', header],
    ['IDAT', deflateSync(Buffer.from(filtered))],
    ['IEND', Buffer.alloc(0)],
  ].map(([type, data]) => {
    const chunk = Buffer.alloc(data.length + 12)
    chunk.writeUInt32BE(data.length)
    chunk.write(type, 4)
    data.copy(chunk, 8)
    return chunk
  })
  return Buffer.concat([Buffer.from('89504e470d0a1a0a', 'hex'), ...chunks])
}

const original = [
  { g: 422, cl: 0, ax: 911, ay: 0, dx: 0, dy: 0 },
  { g: 23, cl: 3, ax: 674, ay: 0, dx: 0, dy: 0 },
]
const subset = original.map((glyph, index) => ({ ...glyph, g: index + 1 }))

test('compares shaping semantics after mapping subset GIDs back to the source', () => {
  assertShapingParity('cff2', original, subset, [0, 422, 23])
  assert.throws(
    () =>
      assertShapingParity(
        'cff2',
        original,
        [{ ...subset[0], ax: 1000 }, subset[1]],
        [0, 422, 23],
      ),
    /positioning changed/u,
  )
  assert.throws(
    () =>
      assertShapingParity(
        'cff2',
        original,
        [{ ...subset[0], cl: 1 }, subset[1]],
        [0, 422, 23],
      ),
    /clusters/u,
  )
  assert.throws(
    () => assertShapingParity('cff2', original, subset.slice(1), [0, 422, 23]),
    /glyphs/u,
  )
})

test('rejects missing glyphs, invalid mappings and empty shaping instead of false parity', () => {
  assert.throws(() => assertShapingParity('empty', [], [], []), /empty/u)
  assert.throws(
    () =>
      assertShapingParity(
        'missing',
        [{ ...original[0], g: 0 }],
        subset,
        [0, 422],
      ),
    /source has missing/u,
  )
  for (const mapping of [[0, null, 23], [0, 0, 23], [0]]) {
    assert.throws(
      () => assertShapingParity('missing', original, subset, mapping),
      /invalid or missing/u,
    )
  }
})

test('decodes all Cairo PNG row filters and compares pixels independently of compression', () => {
  const rows = [
    [200, 50, 20, 10, 80, 240],
    [20, 220, 0, 255, 50, 230],
  ]
  const reference = png({ rows, width: 2 })
  for (let filter = 0; filter <= 4; filter += 1) {
    const image = png({ rows, width: 2, filter })
    assert.deepEqual(pngPixels(image).pixels, Buffer.from(rows.flat()))
    const result = assertRenderingParity('color', reference, image, {
      color: true,
    })
    assert.equal(result.colorPixels, 4)
  }
})

test('rejects blank, monochrome fallback, missing layers and changed image dimensions', () => {
  const white = png({ rows: [[255, 255, 255]] })
  const black = png({ rows: [[0, 0, 0]] })
  const color = png()
  assert.throws(() => assertRenderingParity('blank', white, white), /blank/u)
  assert.throws(
    () => assertRenderingParity('fallback', black, black, { color: true }),
    /did not paint color/u,
  )
  assert.throws(
    () => assertRenderingParity('layer', color, black, { color: true }),
    /pixels changed/u,
  )
  assert.throws(
    () =>
      assertRenderingParity(
        'size',
        color,
        png({ rows: [[200, 50, 20, 200, 50, 20]], width: 2 }),
      ),
    /width changed/u,
  )
  assert.throws(() => pngPixels(color.subarray(0, 20)), /truncated/u)
})
