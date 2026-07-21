import { createHash } from 'node:crypto'
import { readFile, readdir } from 'node:fs/promises'
import { dirname, join, posix, relative, resolve, sep } from 'node:path'
import { pathToFileURL } from 'node:url'

const workspaceRoot = dirname(import.meta.dirname)
const manifestRelativePath = 'fixtures/fonts/manifest.json'
const digestPattern = /^[\da-f]{64}$/u

async function discoverFontPaths(directory, root) {
  const entries = await readdir(directory, { withFileTypes: true })
  const nestedPaths = await Promise.all(
    entries.map(async entry => {
      const path = join(directory, entry.name)

      if (entry.isDirectory()) {
        return discoverFontPaths(path, root)
      }

      return /\.(?:otf|ttf)$/u.test(entry.name)
        ? [relative(root, path).split(sep).join(posix.sep)]
        : []
    }),
  )

  return nestedPaths.flat().toSorted()
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

  const sourceValues = [
    font.source?.project,
    font.source?.url,
    font.source?.license,
    font.source?.licenseUrl,
  ]
  if (
    sourceValues.some(value => typeof value !== 'string' || value.length === 0)
  ) {
    throw new Error(`${font.path} must declare source and license metadata`)
  }
  if (!font.source.url.startsWith('https://')) {
    throw new Error(`${font.path} must use an HTTPS source URL`)
  }
  if (!font.source.licenseUrl.startsWith('https://')) {
    throw new Error(`${font.path} must use an HTTPS license URL`)
  }
}

export async function checkFontFixtures({ root = workspaceRoot } = {}) {
  const manifestPath = join(root, manifestRelativePath)
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'))

  if (manifest.schemaVersion !== 1 || !Array.isArray(manifest.fonts)) {
    throw new Error(`${manifestRelativePath} must use schema version 1`)
  }

  const declaredPaths = manifest.fonts.map(font => font.path)
  const sortedPaths = declaredPaths.toSorted()

  if (new Set(declaredPaths).size !== declaredPaths.length) {
    throw new Error(`${manifestRelativePath} contains duplicate paths`)
  }
  if (declaredPaths.some((path, index) => path !== sortedPaths[index])) {
    throw new Error(`${manifestRelativePath} fonts must be sorted by path`)
  }

  const discoveredPaths = await discoverFontPaths(
    join(root, 'fixtures/fonts'),
    root,
  )

  if (JSON.stringify(declaredPaths) !== JSON.stringify(discoveredPaths)) {
    throw new Error(
      `font fixture inventory mismatch: declared ${declaredPaths.join(', ')}; found ${discoveredPaths.join(', ')}`,
    )
  }

  for (const font of manifest.fonts) {
    assertFixtureMetadata(font)

    const absolutePath = resolve(root, font.path)
    const fontRoot = `${resolve(root, 'fixtures/fonts')}${sep}`

    if (!absolutePath.startsWith(fontRoot)) {
      throw new Error(`font fixture path escapes fixtures/fonts: ${font.path}`)
    }
    if (!digestPattern.test(font.sha256)) {
      throw new Error(`${font.path} has an invalid SHA-256 digest`)
    }

    const contents = await readFile(absolutePath)
    const digest = createHash('sha256').update(contents).digest('hex')

    if (digest !== font.sha256) {
      throw new Error(
        `${font.path} digest is ${digest}; expected ${font.sha256}`,
      )
    }

    const checksum = await readFile(`${absolutePath}.sha256`, 'utf8')
    const expectedChecksum = `${font.sha256}  ${font.path}\n`

    if (checksum !== expectedChecksum) {
      throw new Error(`${font.path}.sha256 does not match the manifest`)
    }

    assertFixtureShape(font, contents)
  }

  return { count: manifest.fonts.length, paths: declaredPaths }
}

const entryPath = process.argv[1]
if (
  entryPath !== undefined &&
  import.meta.url === pathToFileURL(resolve(entryPath)).href
) {
  const result = await checkFontFixtures()
  console.log(`Verified ${result.count} font fixtures.`)
}
