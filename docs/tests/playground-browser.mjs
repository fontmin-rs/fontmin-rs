import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { extname, join, normalize, relative, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium } from 'playwright'

const docsRoot = resolve(fileURLToPath(new URL('..', import.meta.url)))
const workspaceRoot = resolve(docsRoot, '..')
const outputRoot = join(docsRoot, '.vitepress', 'dist')
const fixture = join(
  workspaceRoot,
  'fixtures',
  'fonts',
  'ttf',
  'roboto-regular.ttf',
)

const contentTypes = {
  '.css': 'text/css',
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.mjs': 'text/javascript',
  '.svg': 'image/svg+xml',
  '.wasm': 'application/wasm',
}

function outputPath(pathname) {
  const decoded = decodeURIComponent(pathname)
  const requested = decoded === '/' ? 'index.html' : decoded.slice(1)
  const withExtension = extname(requested) ? requested : `${requested}.html`
  const path = resolve(outputRoot, normalize(withExtension))

  if (relative(outputRoot, path).startsWith('..')) {
    return
  }

  return path
}

const server = createServer(async (request, response) => {
  const path = outputPath(
    new URL(request.url ?? '/', 'http://localhost').pathname,
  )
  if (!path) {
    response.writeHead(403).end()
    return
  }

  try {
    const body = await readFile(path)
    response
      .writeHead(200, {
        'content-type':
          contentTypes[extname(path)] ?? 'application/octet-stream',
      })
      .end(body)
  } catch {
    response.writeHead(404).end()
  }
})

await new Promise(resolveServer => {
  server.listen(0, resolveServer)
})
const address = server.address()
const baseUrl = `http://127.0.0.1:${address.port}`

try {
  const browser = await chromium.launch({ headless: true })
  try {
    const page = await browser.newPage()
    await page.goto(`${baseUrl}/playground`)

    const documentation = page.getByTestId('playground-documentation')
    await documentation.waitFor()
    assert.deepEqual(
      await documentation.evaluate(element => {
        const heading = element.querySelector('h2')
        if (!heading) {
          throw new Error('Playground documentation heading is missing.')
        }

        return {
          headingFontSize: getComputedStyle(heading).fontSize,
          maxWidth: getComputedStyle(element).maxWidth,
        }
      }),
      {
        headingFontSize: '24px',
        maxWidth: '688px',
      },
    )

    const [fileChooser] = await Promise.all([
      page.waitForEvent('filechooser'),
      page.getByTestId('open-file-dialog').click(),
    ])
    await fileChooser.setFiles(fixture)
    await page.locator('#playground-characters').fill('Hello')
    await page.getByTestId('playground-delivery-latin').check()
    await page.getByTestId('playground-delivery-cjk').check()
    await page.getByTestId('generate').click()

    await page
      .getByTestId('download-asset-roboto-regular-latin.woff2')
      .waitFor()
    await page.getByTestId('download-asset-roboto-regular-cjk.woff2').waitFor()
    await expectText(page, 'roboto-regular.css')

    const [latinDownload] = await Promise.all([
      page.waitForEvent('download'),
      page.getByTestId('download-asset-roboto-regular-latin.woff2').click(),
    ])
    assert.equal(
      latinDownload.suggestedFilename(),
      'roboto-regular-latin.woff2',
    )

    const [cjkDownload] = await Promise.all([
      page.waitForEvent('download'),
      page.getByTestId('download-asset-roboto-regular-cjk.woff2').click(),
    ])
    assert.equal(cjkDownload.suggestedFilename(), 'roboto-regular-cjk.woff2')

    const [cssDownload] = await Promise.all([
      page.waitForEvent('download'),
      page.getByTestId('download-asset-roboto-regular.css').click(),
    ])
    const cssPath = await cssDownload.path()
    assert.notEqual(cssPath, null)
    assert.match(
      await readFile(cssPath, 'utf8'),
      /unicode-range: U\+0000-00FF;/u,
    )
    assert.match(
      await readFile(cssPath, 'utf8'),
      /unicode-range: U\+4E00-9FFF;/u,
    )

    const [archiveDownload] = await Promise.all([
      page.waitForEvent('download'),
      page.getByTestId('download-archive').click(),
    ])
    assert.equal(
      archiveDownload.suggestedFilename(),
      'roboto-regular-fontmin.zip',
    )
  } finally {
    await browser.close()
  }
} finally {
  await new Promise(resolveServer => {
    server.close(resolveServer)
  })
}

async function expectText(page, value) {
  await page.getByText(value, { exact: true }).waitFor()
}
