import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { promisify } from 'node:util'

const executeFile = promisify(execFile)
const workspaceRoot = dirname(import.meta.dirname)
const instanceCoordinates = [
  ...[0, 0.5, 1].flatMap(CRSV =>
    [0, 0.5, 1].flatMap(MONO => [0, -15].map(slnt => ({ CRSV, MONO, slnt }))),
  ),
  { CRSV: 1, CASL: 1, wght: 700, slnt: -15, MONO: 0.5 },
  { CRSV: 0, CASL: 0.5, wght: 1000, slnt: -7.5 },
]

async function shape(path, coordinates) {
  const { stdout } = await executeFile(
    'hb-shape',
    [
      path,
      // Cursive alternates, kerning, and combining marks exercise both layout tables.
      'ag AV To a\u0301g\u0308',
      '--shapers=ot',
      '--font-funcs=ot',
      '--script=Latn',
      '--language=en',
      '--direction=ltr',
      '--features=kern,mark,mkmk',
      '--no-glyph-names',
      '--output-format=json',
      `--variations=${Object.entries(coordinates)
        .map(([tag, value]) => `${tag}=${value}`)
        .join(',')}`,
    ],
    { timeout: 30_000 },
  )
  return JSON.parse(stdout)
}

async function assertLayoutDiagnostics(root, source, native, binding, wasm) {
  const contract = JSON.parse(
    await readFile(join(root, 'contracts/public-api.json'), 'utf8'),
  ).instancing.glyfLayout
  let gsub
  for (let index = 0; index < source.readUInt16BE(4); index += 1) {
    const record = 12 + index * 16
    if (source.toString('ascii', record, record + 4) === 'GSUB') {
      gsub = source.readUInt32BE(record + 8)
    }
  }
  assert.ok(gsub !== undefined)
  const variations = gsub + source.readUInt32BE(gsub + 10)
  const conditions = variations + source.readUInt32BE(variations + 8)
  const condition = conditions + source.readUInt32BE(conditions + 2)
  const unsupported = Buffer.from(source)
  unsupported.writeUInt16BE(99, condition)
  const malformed = Buffer.from(source)
  malformed.writeUInt32BE(source.length, gsub + 10)
  for (const [input, code] of [
    [unsupported, contract.unsupportedDiagnostic],
    [malformed, contract.malformedDiagnostic],
  ]) {
    assert.throws(() => native.instantiateFont(input), { code })
    await assert.rejects(() => wasm.instantiateFont(input), { code })
    await assert.rejects(
      () => binding.instantiateFontAsync(input),
      error => error.message.includes(code),
    )
  }
}

export async function runInstanceShaping({ root = workspaceRoot } = {}) {
  const sourcePath = join(root, 'fixtures/fonts/ttf/recursive-variable.ttf')
  const source = await readFile(sourcePath)
  const sourceSha256 = createHash('sha256').update(source).digest('hex')
  assert.equal(
    sourceSha256,
    '653221ca467f4732fe6856ac493f6c409e9f56a7674abe36b2364acc89796f7c',
  )
  const native = await import(
    pathToFileURL(join(root, 'packages/fontmin/dist/index.mjs')).href
  )
  const binding = await import(
    pathToFileURL(join(root, 'napi/fontmin/src-js/bindings.js')).href
  )
  const wasm = await import(
    pathToFileURL(join(root, 'wasm/fontmin/dist/index.mjs')).href
  )
  if (!wasm.isWasmInitialized()) {
    await wasm.initWasm(
      await readFile(join(root, 'wasm/fontmin/dist/fontmin_wasm_core_bg.wasm')),
    )
  }
  const { stdout } = await executeFile('hb-shape', ['--version'])
  const report = {
    status: 'running',
    sourceSha256,
    shaper: stdout.trim(),
    cases: [],
  }
  const directory = join(root, 'benchmarks/rendering-current')
  await mkdir(directory, { recursive: true })
  const temporary = await mkdtemp(join(tmpdir(), 'fontmin-instance-shaping-'))
  try {
    await assertLayoutDiagnostics(root, source, native, binding, wasm)
    report.diagnostics = 'passed'
    for (const variationCoordinates of instanceCoordinates) {
      const expected = await shape(sourcePath, variationCoordinates)
      assert.ok(
        expected.every(glyph => glyph.g > 0),
        'source has missing glyphs',
      )
      const nativeOutput = native.instantiateFont(source, {
        variationCoordinates,
      })
      for (const [runtime, output] of [
        ['native', nativeOutput],
        [
          'native-async',
          await binding.instantiateFontAsync(source, {
            variationCoordinates,
          }),
        ],
        ['wasm', await wasm.instantiateFont(source, { variationCoordinates })],
      ]) {
        assert.deepEqual(
          Buffer.from(output),
          Buffer.from(nativeOutput),
          `${runtime}: byte parity`,
        )
        const outputPath = join(temporary, `${runtime}.ttf`)
        await writeFile(outputPath, output)
        const actual = await shape(outputPath, {})
        assert.deepEqual(
          actual,
          expected,
          `${runtime} ${JSON.stringify(variationCoordinates)}: glyphs, clusters, advances, offsets`,
        )
        report.cases.push({ runtime, variationCoordinates, shape: actual })
      }
      console.log(
        `Verified Recursive instance ${JSON.stringify(variationCoordinates)}`,
      )
    }
    report.status = 'passed'
    return report
  } catch (error) {
    report.status = 'failed'
    report.error = error.message
    throw error
  } finally {
    await writeFile(
      join(directory, 'instance-shaping.json'),
      `${JSON.stringify(report, null, 2)}\n`,
    )
    await rm(temporary, { recursive: true, force: true })
  }
}

if (
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  await runInstanceShaping()
}
