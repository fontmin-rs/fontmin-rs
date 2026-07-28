import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import test from 'node:test'
import { pathToFileURL } from 'node:url'

const workspaceRoot = dirname(import.meta.dirname)
const contract = JSON.parse(
  await readFile(join(workspaceRoot, 'contracts/public-api.json'), 'utf8'),
)

async function readJson(path) {
  return JSON.parse(await readFile(join(workspaceRoot, path), 'utf8'))
}

async function runtimeExports(path) {
  const module = await import(pathToFileURL(join(workspaceRoot, path)).href)
  return Object.keys(module).toSorted()
}

function camelCase(value) {
  return value.replaceAll(/_[a-z]/gu, match => match[1].toUpperCase())
}

function exportedInterfaceFields(source, name) {
  const body = source.match(
    new RegExp(
      String.raw`export interface ${name}[^{]*\{(?<body>[\s\S]*?)\n\}`,
      'u',
    ),
  )?.groups?.body

  assert.ok(body, `expected exported interface ${name}`)
  return [...body.matchAll(/^\s{2}(?<field>[A-Za-z][A-Za-z0-9]*)\??:/gmu)]
    .map(match => match.groups.field)
    .toSorted()
}

function rustStructFields(source, name) {
  const body = source.match(
    new RegExp(String.raw`pub struct ${name} \{(?<body>[\s\S]*?)\n\}`, 'u'),
  )?.groups?.body

  assert.ok(body, `expected Rust struct ${name}`)
  return [...body.matchAll(/^\s+pub (?<field>[a-z][a-z0-9_]*):/gmu)]
    .map(match => camelCase(match.groups.field))
    .toSorted()
}

test('freezes Node and WASM runtime exports and package subpaths', async () => {
  const [nodeManifest, wasmManifest] = await Promise.all([
    readJson('packages/fontmin/package.json'),
    readJson('wasm/fontmin/package.json'),
  ])
  const entryPoints = [
    ['node', '.', 'packages/fontmin/dist/index.mjs'],
    ['node', './compat', 'packages/fontmin/dist/compat.mjs'],
    ['node', './plugins', 'packages/fontmin/dist/plugins.mjs'],
    ['node', './presets', 'packages/fontmin/dist/presets.mjs'],
    ['wasm', '.', 'wasm/fontmin/dist/index.mjs'],
  ]

  for (const [runtime, subpath, path] of entryPoints) {
    assert.deepEqual(
      await runtimeExports(path),
      contract.exports[runtime][subpath],
      `${runtime} ${subpath} exports changed; update the public contract and changelog intentionally`,
    )
  }

  assert.deepEqual(Object.keys(nodeManifest.exports).toSorted(), [
    '.',
    './compat',
    './package.json',
    './plugins',
    './presets',
  ])
  assert.deepEqual(Object.keys(wasmManifest.exports), ['.'])
  assert.deepEqual(nodeManifest.bin, {
    'fontmin-rs': './bin/fontmin-rs.mjs',
  })
})

test('freezes the Rust, Node, and browser configuration boundaries', async () => {
  const [rustConfig, rustLoader, nodeTypes, browserOptimize] =
    await Promise.all([
      readFile(
        join(workspaceRoot, 'crates/fontmin_config/src/config.rs'),
        'utf8',
      ),
      readFile(join(workspaceRoot, 'apps/fontmin/src/config.rs'), 'utf8'),
      readFile(join(workspaceRoot, 'packages/fontmin/src/types.ts'), 'utf8'),
      readFile(join(workspaceRoot, 'wasm/fontmin/src/optimize.ts'), 'utf8'),
    ])

  assert.deepEqual(
    rustStructFields(rustConfig, 'FontminConfig'),
    contract.config.rust.fields,
  )
  assert.deepEqual(
    exportedInterfaceFields(nodeTypes, 'FontminConfig'),
    contract.config.node.fields,
  )
  assert.deepEqual(
    exportedInterfaceFields(browserOptimize, 'BrowserOptimizeConfig'),
    contract.config.browser.fields,
  )
  assert.deepEqual(
    exportedInterfaceFields(browserOptimize, 'BrowserAsset'),
    contract.config.browser.assetFields,
  )

  const extensions = [
    ...new Set(
      [...rustLoader.matchAll(/"fontmin\.config\.(?<extension>[a-z]+)"/gu)].map(
        match => match.groups.extension,
      ),
    ),
  ].toSorted()
  assert.deepEqual(extensions, contract.config.rust.moduleExtensions)

  for (const field of contract.config.sharedSubsetFields) {
    const rustField = field.replaceAll(
      /[A-Z]/gu,
      letter => `_${letter.toLowerCase()}`,
    )
    assert.match(
      `${rustConfig}\n${nodeTypes}`,
      new RegExp(String.raw`\b${rustField}\b|\b${field}\b`, 'u'),
      `shared subset field ${field} is no longer represented`,
    )
  }
})

test('freezes diagnostic codes and plugin lifecycle hooks', async () => {
  const [diagnostics, nodeTypes] = await Promise.all([
    readFile(
      join(workspaceRoot, 'crates/fontmin_diagnostics/src/lib.rs'),
      'utf8',
    ),
    readFile(join(workspaceRoot, 'packages/fontmin/src/types.ts'), 'utf8'),
  ])
  const codes = [
    ...new Set(
      [...diagnostics.matchAll(/"(?<code>fontmin::[a-z_]+)"/gu)].map(
        match => match.groups.code,
      ),
    ),
  ].toSorted()

  assert.deepEqual(codes, contract.diagnosticCodes)
  for (const hook of contract.pluginLifecycle) {
    assert.match(nodeTypes, new RegExp(String.raw`\b${hook}\?\(`, 'u'))
  }
})

test('keeps the public contract deterministic and documented', async () => {
  const [english, chinese] = await Promise.all([
    readFile(join(workspaceRoot, 'docs/contracts.md'), 'utf8'),
    readFile(join(workspaceRoot, 'docs/zh/contracts.md'), 'utf8'),
  ])

  assert.equal(contract.schemaVersion, 1)
  assert.deepEqual(contract.cli.exitCodes, { error: 1, success: 0 })
  assert.deepEqual(contract.fileNaming, {
    default: '{stem}.{extension}',
    deliverySlice: '{stem}-{slice}.{extension}',
    iconFontDefaultStem: 'iconfont',
    preserveOriginal: '{input-file-name}',
  })

  for (const document of [english, chinese]) {
    assert.match(document, /contracts\/public-api\.json/u)
    assert.match(document, /buildStart/u)
    assert.match(document, /fontmin::invalid_font/u)
    assert.match(document, /\{stem\}-\{slice\}\.\{extension\}/u)
  }
})

test('keeps the published release state consistent across public surfaces', async () => {
  const nodeManifest = await readJson('packages/fontmin/package.json')
  const isPrerelease = nodeManifest.version.includes('-')
  const publicDocumentPaths = [
    'README.md',
    'packages/fontmin/README.md',
    'docs/index.md',
    'docs/zh/index.md',
    'docs/guide/getting-started.md',
    'docs/zh/guide/getting-started.md',
    'docs/guide/migration.md',
    'docs/zh/guide/migration.md',
    'docs/api/wasm.md',
    'docs/zh/api/wasm.md',
  ]

  assert.equal(
    contract.stability,
    isPrerelease ? 'release-candidate' : 'stable',
  )

  if (!isPrerelease) {
    for (const documentPath of publicDocumentPaths) {
      const document = await readFile(join(workspaceRoot, documentPath), 'utf8')
      assert.doesNotMatch(
        document,
        /(?:fontmin-rs|@fontmin-rs\/wasm)@(?:alpha|beta|next|rc)\b/u,
        `${documentPath} must install the stable release without a prerelease dist-tag`,
      )
    }
  }
})
