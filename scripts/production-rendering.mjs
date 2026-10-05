import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'
import { inflateSync } from 'node:zlib'
import { prepareProductionFixtures } from './prepare-production-fixtures.mjs'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)

function digest(bytes) {
  return createHash('sha256').update(bytes).digest('hex')
}

export function assertShapingParity(label, original, subset, newToOld) {
  assert.ok(original.length > 0, `${label}: empty shaping result`)
  assert.ok(
    original.every(glyph => glyph.g !== 0),
    `${label}: source has missing glyphs`,
  )
  const mapped = subset.map(glyph => {
    const oldGid = newToOld[glyph.g]
    assert.ok(
      Number.isSafeInteger(oldGid) && oldGid > 0,
      `${label}: invalid or missing subset GID ${glyph.g}`,
    )
    return { ...glyph, g: oldGid }
  })
  assert.deepEqual(
    mapped,
    original,
    `${label}: glyphs, clusters, or positioning changed`,
  )
}

function paeth(left, up, upperLeft) {
  const prediction = left + up - upperLeft
  const distances = [
    Math.abs(prediction - left),
    Math.abs(prediction - up),
    Math.abs(prediction - upperLeft),
  ]
  if (distances[0] <= distances[1] && distances[0] <= distances[2]) {
    return left
  }
  return distances[1] <= distances[2] ? up : upperLeft
}

// Decode only the non-interlaced, 8-bit PNG variants emitted by Cairo. This
// prevents equal blank/monochrome fallback images from passing the color gate.
export function pngPixels(png) {
  assert.equal(png.subarray(0, 8).toString('hex'), '89504e470d0a1a0a')
  const chunks = []
  let header
  for (let offset = 8; offset < png.length;) {
    const length = png.readUInt32BE(offset)
    assert.ok(offset + length + 12 <= png.length, 'truncated PNG chunk')
    const type = png.toString('ascii', offset + 4, offset + 8)
    const data = png.subarray(offset + 8, offset + 8 + length)
    if (type === 'IHDR') {
      header = data
    }
    if (type === 'IDAT') {
      chunks.push(data)
    }
    offset += length + 12
  }
  assert.ok(header?.length === 13, 'missing PNG header')
  const width = header.readUInt32BE(0)
  const height = header.readUInt32BE(4)
  const channels = { 0: 1, 2: 3, 4: 2, 6: 4 }[header[9]]
  assert.ok(
    channels !== undefined && header[8] === 8 && header[12] === 0,
    'unsupported Cairo PNG format',
  )
  const stride = width * channels
  const filtered = inflateSync(Buffer.concat(chunks))
  assert.equal(filtered.length, (stride + 1) * height)
  const pixels = Buffer.alloc(stride * height)
  for (let y = 0; y < height; y += 1) {
    const filter = filtered[y * (stride + 1)]
    assert.ok(filter <= 4, 'invalid PNG row filter')
    for (let x = 0; x < stride; x += 1) {
      const index = y * stride + x
      const left = x < channels ? 0 : pixels[index - channels]
      const up = y === 0 ? 0 : pixels[index - stride]
      const upperLeft =
        y === 0 || x < channels ? 0 : pixels[index - stride - channels]
      const predictors = [
        0,
        left,
        up,
        Math.floor((left + up) / 2),
        paeth(left, up, upperLeft),
      ]
      pixels[index] =
        (filtered[y * (stride + 1) + x + 1] + predictors[filter]) % 256
    }
  }
  let inkPixels = 0
  let colorPixels = 0
  for (let offset = 0; offset < pixels.length; offset += channels) {
    const rgb =
      channels < 3
        ? [pixels[offset], pixels[offset], pixels[offset]]
        : [...pixels.subarray(offset, offset + 3)]
    const alpha =
      channels === 2 || channels === 4 ? pixels[offset + channels - 1] : 255
    if (alpha > 0 && Math.min(...rgb) < 250) {
      inkPixels += 1
    }
    if (alpha > 0 && Math.max(...rgb) - Math.min(...rgb) > 16) {
      colorPixels += 1
    }
  }
  return { width, height, pixels, inkPixels, colorPixels }
}

export function assertRenderingParity(
  label,
  original,
  subset,
  { color = false } = {},
) {
  const source = pngPixels(original)
  const output = pngPixels(subset)
  assert.ok(source.inkPixels > 0, `${label}: source rendering is blank`)
  if (color) {
    assert.ok(
      source.colorPixels > 0,
      `${label}: renderer did not paint color layers`,
    )
  }
  assert.equal(output.width, source.width, `${label}: rendered width changed`)
  assert.equal(
    output.height,
    source.height,
    `${label}: rendered height changed`,
  )
  assert.ok(
    output.pixels.equals(source.pixels),
    `${label}: rendered pixels changed`,
  )
  return {
    width: source.width,
    height: source.height,
    inkPixels: source.inkPixels,
    colorPixels: source.colorPixels,
    sha256: digest(source.pixels),
  }
}

async function shapeAndRender(fontPath, sample, imagePath) {
  const common = [
    `--font-file=${fontPath}`,
    `--text=${sample.text}`,
    '--shapers=ot',
    '--font-funcs=ot',
    `--language=${sample.language}`,
    `--variations=${sample.variations ?? ''}`,
  ]
  const { stdout } = await executeFile(
    'hb-shape',
    [...common, '--output-format=json', '--no-glyph-names'],
    { timeout: 30_000 },
  )
  await executeFile(
    'hb-view',
    [
      ...common,
      '--font-size=64',
      '--background=ffffff',
      '--foreground=000001',
      '--output-format=png',
      `--output-file=${imagePath}`,
    ],
    { timeout: 30_000 },
  )
  return { shape: JSON.parse(stdout), image: await readFile(imagePath) }
}

async function subsetRenderingFont(api, fixture, contents, text, label) {
  const result = await api.subsetTtfWithReport(contents, {
    text,
    keepLayout: 'conservative',
    missingGlyphs: 'error',
  })
  assert.ok(
    result.report.glyphsRetained < fixture.expected.glyphCount,
    `${label}: font was not subset`,
  )
  const info = await api.inspect(result.data)
  for (const table of fixture.expected.tables) {
    assert.ok(info.metadata.tables.includes(table), `${label}: lost ${table}`)
  }
  return result
}

function assertRuntimeParity(result, reference, label) {
  if (reference !== undefined) {
    assert.deepEqual(
      result.report,
      reference.report,
      `${label}: runtime mapping/report mismatch`,
    )
    assert.ok(
      Buffer.from(result.data).equals(reference.data),
      `${label}: runtime output mismatch`,
    )
  }
  return result
}

export async function runProductionRendering({ root = workspaceRoot } = {}) {
  const directory = join(root, 'benchmarks/rendering-current')
  await mkdir(directory, { recursive: true })
  const reportPath = join(directory, 'report.json')
  const report = { schemaVersion: 1, status: 'running', tools: {}, cases: [] }
  await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`)
  let temporary
  try {
    temporary = await mkdtemp(join(tmpdir(), 'fontmin-rendering-'))
    const prepared = await prepareProductionFixtures({ root })
    const manifest = JSON.parse(
      await readFile(join(root, 'fixtures/production/manifest.json'), 'utf8'),
    )
    const native = await import(
      pathToFileURL(join(root, 'packages/fontmin/dist/index.mjs')).href
    )
    const wasm = await import(
      pathToFileURL(join(root, 'wasm/fontmin/dist/index.mjs')).href
    )
    if (!wasm.isWasmInitialized()) {
      await wasm.initWasm(
        await readFile(
          join(root, 'wasm/fontmin/dist/fontmin_wasm_core_bg.wasm'),
        ),
      )
    }
    for (const command of ['hb-shape', 'hb-view']) {
      const { stdout } = await executeFile(command, ['--version'])
      report.tools[command] = stdout.trim()
    }
    for (const fixture of manifest.fixtures.filter(entry =>
      entry.scenarios.includes('shape-render'),
    )) {
      const inputPath = prepared.fixtures.find(
        entry => entry.id === fixture.id,
      )?.path
      assert.ok(inputPath, `${fixture.id}: fixture was not prepared`)
      const contents = await readFile(inputPath)
      for (const sample of fixture.rendering) {
        const label = `${fixture.id}-${sample.name}`
        const entry = {
          fixture: fixture.id,
          sourceSha256: fixture.sha256,
          sample,
          runtimes: [],
          status: 'failed',
        }
        report.cases.push(entry)
        const original = await shapeAndRender(
          inputPath,
          sample,
          join(directory, `${label}-source.png`),
        )
        entry.sourceShape = original.shape
        let nativeResult
        for (const [runtime, api] of [
          ['native', native],
          ['wasm', wasm],
        ]) {
          const result = await subsetRenderingFont(
            api,
            fixture,
            contents,
            sample.text,
            label,
          )
          nativeResult = assertRuntimeParity(result, nativeResult, label)
          const outputPath = join(temporary, `${label}-${runtime}.otf`)
          await writeFile(outputPath, result.data)
          const output = await shapeAndRender(
            outputPath,
            sample,
            join(directory, `${label}-${runtime}.png`),
          )
          const runtimeReport = {
            runtime,
            glyphsRetained: result.report.glyphsRetained,
            droppedContextSubtables: result.report.droppedContextSubtables,
            shape: output.shape,
            newToOld: result.report.newToOld,
            status: 'failed',
          }
          entry.runtimes.push(runtimeReport)
          assertShapingParity(
            `${label} ${runtime}`,
            original.shape,
            output.shape,
            result.report.newToOld,
          )
          runtimeReport.rendering = assertRenderingParity(
            `${label} ${runtime}`,
            original.image,
            output.image,
            sample,
          )
          runtimeReport.status = 'passed'
        }
        entry.status = 'passed'
      }
    }
    assert.ok(report.cases.length > 0, 'no production rendering cases declared')
    report.status = 'passed'
    return report
  } catch (error) {
    report.status = 'failed'
    report.error = error.message
    throw error
  } finally {
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`)
    if (temporary !== undefined) {
      await rm(temporary, { force: true, recursive: true })
    }
  }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const result = await runProductionRendering()
  console.log(
    `Verified ${result.cases.length} production shaping/rendering cases across native and WASM.`,
  )
}
