import { createHash } from 'node:crypto'
import { readFile, readdir } from 'node:fs/promises'
import {
  basename,
  dirname,
  join,
  posix,
  relative,
  resolve,
  sep,
} from 'node:path'
import { pathToFileURL } from 'node:url'

const workspaceRoot = dirname(import.meta.dirname)
const fontManifestRelativePath = 'fixtures/fonts/manifest.json'
const malformedManifestRelativePath = 'fixtures/malformed/manifest.json'
const productionManifestRelativePath = 'fixtures/production/manifest.json'
const digestPattern = /^[\da-f]{64}$/u
const gitObjectPattern = /^[\da-f]{40}$/u
const diagnosticCodePattern = /^fontmin::[a-z_]+$/u
const fixtureIdPattern = /^[a-z0-9]+(?:-[a-z0-9]+)*$/u
const immutableGitHubRefPattern = /\/[\da-f]{40}\//u
const immutableCdnRefPattern = /@(?<ref>[\da-f]{40})\//u
const productionScenarios = new Set([
  'inspect',
  'mixed-delivery',
  'performance',
])

async function discoverPaths(directory, root, matches) {
  const entries = await readdir(directory, { withFileTypes: true })
  const nestedPaths = await Promise.all(
    entries.map(async entry => {
      const path = join(directory, entry.name)

      if (entry.isDirectory()) {
        return discoverPaths(path, root, matches)
      }

      return matches(entry.name)
        ? [relative(root, path).split(sep).join(posix.sep)]
        : []
    }),
  )

  return nestedPaths.flat().toSorted()
}

function assertInventory(manifestPath, declaredPaths, discoveredPaths) {
  const sortedPaths = declaredPaths.toSorted()

  if (new Set(declaredPaths).size !== declaredPaths.length) {
    throw new Error(`${manifestPath} contains duplicate paths`)
  }
  if (declaredPaths.some((path, index) => path !== sortedPaths[index])) {
    throw new Error(`${manifestPath} entries must be sorted by path`)
  }
  if (JSON.stringify(declaredPaths) !== JSON.stringify(discoveredPaths)) {
    throw new Error(
      `${manifestPath} inventory mismatch: declared ${declaredPaths.join(', ')}; found ${discoveredPaths.join(', ')}`,
    )
  }
}

function assertHttpsUrl(value, label, path) {
  let url

  try {
    url = new URL(value)
  } catch {
    throw new Error(`${path} must declare a valid ${label} URL`)
  }

  if (url.protocol !== 'https:') {
    throw new Error(`${path} must use an HTTPS ${label} URL`)
  }
  if (
    (url.hostname === 'github.com' ||
      url.hostname === 'raw.githubusercontent.com') &&
    !immutableGitHubRefPattern.test(url.pathname)
  ) {
    throw new Error(`${path} must pin ${label} to an immutable GitHub commit`)
  }
}

function assertSourceMetadata(source, path, { requireGenerator = false } = {}) {
  const sourceValues = [
    source?.project,
    source?.url,
    source?.license,
    source?.licenseUrl,
  ]

  if (
    sourceValues.some(value => typeof value !== 'string' || value.length === 0)
  ) {
    throw new Error(`${path} must declare source and license metadata`)
  }

  assertHttpsUrl(source.url, 'source', path)
  assertHttpsUrl(source.licenseUrl, 'license', path)

  if (!requireGenerator) {
    return
  }
  if (source.kind !== 'synthetic' && source.kind !== 'third-party') {
    throw new Error(`${path} must declare a supported malformed source kind`)
  }
  if (typeof source.generator !== 'string' || source.generator.length === 0) {
    throw new Error(`${path} must describe how the malformed fixture was made`)
  }
}

function assertFixtureShape(font, contents) {
  const signature = contents.subarray(0, 4)
  const outlineTag = font.outlines.startsWith('cff')
    ? font.outlines.toUpperCase()
    : font.outlines

  if (
    font.container === 'ttf' &&
    !signature.equals(Buffer.from([0, 1, 0, 0]))
  ) {
    throw new Error(`${font.path} does not have a TrueType signature`)
  }
  if (font.container === 'otf' && !signature.equals(Buffer.from('OTTO'))) {
    throw new Error(`${font.path} does not have an OpenType signature`)
  }
  if (!contents.includes(Buffer.from(outlineTag))) {
    throw new Error(`${font.path} does not contain the ${font.outlines} table`)
  }
  if (
    font.variation === 'variable' &&
    !contents.includes(Buffer.from('fvar'))
  ) {
    throw new Error(`${font.path} does not contain an fvar table`)
  }
}

function assertFixtureMetadata(font) {
  if (!['otf', 'ttf'].includes(font.container)) {
    throw new Error(`${font.path} has an unsupported container`)
  }
  if (!['cff', 'cff2', 'glyf'].includes(font.outlines)) {
    throw new Error(`${font.path} has an unsupported outline type`)
  }
  if (!['static', 'variable'].includes(font.variation)) {
    throw new Error(`${font.path} has an unsupported variation type`)
  }
  if (!font.path.endsWith(`.${font.container}`)) {
    throw new Error(`${font.path} does not match its declared container`)
  }
  if (
    !Array.isArray(font.coverage) ||
    font.coverage.length === 0 ||
    font.coverage.some(value => typeof value !== 'string' || value.length === 0)
  ) {
    throw new Error(`${font.path} must declare non-empty coverage labels`)
  }

  assertSourceMetadata(font.source, font.path)

  if (font.derivation !== undefined) {
    if (!digestPattern.test(font.derivation.sourceSha256 ?? '')) {
      throw new Error(`${font.path} has an invalid derivation source digest`)
    }
    if (
      !Array.isArray(font.derivation.steps) ||
      font.derivation.steps.length === 0 ||
      font.derivation.steps.some(
        step => typeof step !== 'string' || step.length === 0,
      )
    ) {
      throw new Error(`${font.path} must declare non-empty derivation steps`)
    }
  }
}

function assertStringArray(values, label, path) {
  if (
    !Array.isArray(values) ||
    values.length === 0 ||
    values.some(value => typeof value !== 'string' || value.length === 0) ||
    new Set(values).size !== values.length
  ) {
    throw new Error(`${path} must declare unique, non-empty ${label}`)
  }
}

function assertProductionMetadata(fixture) {
  if (!fixtureIdPattern.test(fixture.id ?? '')) {
    throw new Error(`${fixture.cachePath} must declare a kebab-case fixture id`)
  }
  if (
    typeof fixture.cachePath !== 'string' ||
    basename(fixture.cachePath) !== fixture.cachePath ||
    !/\.(?:otf|ttf)$/u.test(fixture.cachePath)
  ) {
    throw new Error(`${fixture.id} must declare a single font cache path`)
  }
  if (
    !Number.isSafeInteger(fixture.byteLength) ||
    fixture.byteLength <= 0 ||
    !digestPattern.test(fixture.sha256 ?? '') ||
    !gitObjectPattern.test(fixture.gitBlobSha ?? '')
  ) {
    throw new Error(`${fixture.id} must declare immutable byte metadata`)
  }

  assertStringArray(fixture.coverage, 'coverage labels', fixture.id)
  assertStringArray(fixture.scenarios, 'scenarios', fixture.id)

  for (const scenario of fixture.scenarios) {
    if (!productionScenarios.has(scenario)) {
      throw new Error(`${fixture.id} declares unsupported scenario ${scenario}`)
    }
  }

  if (
    typeof fixture.expected?.familyName !== 'string' ||
    fixture.expected.familyName.length === 0 ||
    !Number.isSafeInteger(fixture.expected.glyphCount) ||
    fixture.expected.glyphCount <= 0
  ) {
    throw new Error(`${fixture.id} must declare expected font metadata`)
  }
  assertStringArray(fixture.expected.tables, 'expected tables', fixture.id)
  if (fixture.scenarios.includes('mixed-delivery')) {
    assertStringArray(
      fixture.expected.deliveryTables,
      'expected delivery tables',
      fixture.id,
    )
  }

  assertHttpsUrl(fixture.downloadUrl, 'download', fixture.id)
  assertSourceMetadata(fixture.source, fixture.id)

  const downloadRef = immutableCdnRefPattern.exec(fixture.downloadUrl)?.groups
    ?.ref
  const sourceRef = fixture.source.url.match(/\/(?<ref>[\da-f]{40})\//u)?.groups
    ?.ref

  if (downloadRef === undefined || downloadRef !== sourceRef) {
    throw new Error(
      `${fixture.id} must pin its download mirror to the upstream source commit`,
    )
  }
}

function assertMalformedMetadata(testCase) {
  if (
    typeof testCase.operation !== 'string' ||
    testCase.operation.length === 0
  ) {
    throw new Error(`${testCase.path} must declare a public operation`)
  }
  if (testCase.encoding !== undefined && testCase.encoding !== 'hex') {
    throw new Error(`${testCase.path} has an unsupported encoding`)
  }
  if (
    (testCase.encoding === 'hex') !== testCase.path.endsWith('.hex') ||
    (testCase.encoding === undefined && !testCase.path.endsWith('.bin'))
  ) {
    throw new Error(`${testCase.path} does not match its declared encoding`)
  }

  assertSourceMetadata(testCase.source, testCase.path, {
    requireGenerator: true,
  })

  if (
    !diagnosticCodePattern.test(testCase.expectedDiagnostic?.code ?? '') ||
    typeof testCase.expectedDiagnostic?.message !== 'string' ||
    testCase.expectedDiagnostic.message.length === 0
  ) {
    throw new Error(`${testCase.path} must declare a stable diagnostic`)
  }
}

async function verifyFixtureDigest(root, fixture, fixtureRoot) {
  const absolutePath = resolve(root, fixture.path)
  const allowedRoot = `${resolve(root, fixtureRoot)}${sep}`

  if (!absolutePath.startsWith(allowedRoot)) {
    throw new Error(`${fixture.path} escapes ${fixtureRoot}`)
  }
  if (!digestPattern.test(fixture.sha256)) {
    throw new Error(`${fixture.path} has an invalid SHA-256 digest`)
  }

  const contents = await readFile(absolutePath)
  const digest = createHash('sha256').update(contents).digest('hex')

  if (digest !== fixture.sha256) {
    throw new Error(
      `${fixture.path} digest is ${digest}; expected ${fixture.sha256}`,
    )
  }

  const checksum = await readFile(`${absolutePath}.sha256`, 'utf8')
  const normalizedChecksum = checksum.replaceAll('\r\n', '\n')
  const expectedChecksum = `${fixture.sha256}  ${fixture.path}\n`

  if (normalizedChecksum !== expectedChecksum) {
    throw new Error(`${fixture.path}.sha256 does not match the manifest`)
  }

  return contents
}

export async function checkFontFixtures({ root = workspaceRoot } = {}) {
  const manifestPath = join(root, fontManifestRelativePath)
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))

  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.fonts)) {
    throw new Error(`${fontManifestRelativePath} must use schema version 1`)
  }

  const declaredPaths = manifest.fonts.map(font => font.path)
  const discoveredPaths = await discoverPaths(
    join(root, 'fixtures/fonts'),
    root,
    name => /\.(?:otf|ttf)$/u.test(name),
  )

  assertInventory(fontManifestRelativePath, declaredPaths, discoveredPaths)

  for (const font of manifest.fonts) {
    assertFixtureMetadata(font)
    const contents = await verifyFixtureDigest(root, font, 'fixtures/fonts')

    assertFixtureShape(font, contents)
  }

  const productionManifest = JSON.parse(
    await readFile(join(root, productionManifestRelativePath), 'utf8'),
  )

  if (
    productionManifest.schemaVersion !== 1 ||
    !Array.isArray(productionManifest.fixtures)
  ) {
    throw new Error(
      `${productionManifestRelativePath} must use schema version 1`,
    )
  }

  const productionPaths = productionManifest.fixtures.map(
    fixture => fixture.cachePath,
  )

  assertInventory(
    productionManifestRelativePath,
    productionPaths,
    productionPaths.toSorted(),
  )

  for (const fixture of productionManifest.fixtures) {
    assertProductionMetadata(fixture)
  }

  const malformedManifestPath = join(root, malformedManifestRelativePath)
  const malformedManifest = JSON.parse(
    await readFile(malformedManifestPath, 'utf8'),
  )

  if (
    malformedManifest.schemaVersion !== 1 ||
    !Array.isArray(malformedManifest.cases)
  ) {
    throw new Error(
      `${malformedManifestRelativePath} must use schema version 1`,
    )
  }

  const malformedPaths = malformedManifest.cases.map(testCase => testCase.path)
  const discoveredMalformedPaths = await discoverPaths(
    join(root, 'fixtures/malformed'),
    root,
    name => /\.(?:bin|hex)$/u.test(name),
  )

  assertInventory(
    malformedManifestRelativePath,
    malformedPaths,
    discoveredMalformedPaths,
  )

  for (const testCase of malformedManifest.cases) {
    assertMalformedMetadata(testCase)
    const contents = await verifyFixtureDigest(
      root,
      testCase,
      'fixtures/malformed',
    )

    if (testCase.encoding === 'hex') {
      const hex = contents.toString('utf8').trim()

      if (!/^(?:[\da-f]{2})+$/u.test(hex)) {
        throw new Error(
          `${testCase.path} must contain complete lowercase hex bytes`,
        )
      }
    }
  }

  return {
    count: manifest.fonts.length,
    malformedCount: malformedManifest.cases.length,
    malformedPaths,
    paths: declaredPaths,
    productionCount: productionManifest.fixtures.length,
    productionIds: productionManifest.fixtures.map(fixture => fixture.id),
  }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const result = await checkFontFixtures()
  console.log(
    `Verified ${result.count} checked-in fonts, ${result.productionCount} production fixtures, and ${result.malformedCount} malformed fixtures.`,
  )
}
