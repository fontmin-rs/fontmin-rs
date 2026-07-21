import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { extname, resolve } from 'node:path'
import { chromium, firefox, webkit } from 'playwright'

const browserName = process.env.BROWSER ?? 'chromium'
const launcher = { chromium, firefox, webkit }[browserName]
const root = resolve(import.meta.dirname, '../../..')
const server = createServer(async (request, response) => {
  const pathname = new URL(request.url ?? '/', 'http://localhost').pathname
  if (pathname === '/') {
    response
      .writeHead(200, { 'content-type': 'text/html' })
      .end('<!doctype html>')
    return
  }
  const path = resolve(root, `.${pathname}`)

  if (!path.startsWith(root)) {
    response.writeHead(403).end()
    return
  }

  try {
    const body = await readFile(path)
    const extension = extname(path)
    let type = 'application/octet-stream'
    if (extension === '.wasm') {
      type = 'application/wasm'
    } else if (extension === '.mjs') {
      type = 'text/javascript'
    }
    response.writeHead(200, { 'content-type': type }).end(body)
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
  const browser = await launcher.launch({ headless: true })
  try {
    const page = await browser.newPage()
    await page.goto(baseUrl)
    const result = await page.evaluate(async base => {
      const fontmin = await import(`${base}/wasm/fontmin/dist/index.mjs`)
      const response = await fetch(
        `${base}/fixtures/fonts/ttf/roboto-regular.ttf`,
      )
      const ttf = new Uint8Array(await response.arrayBuffer())

      await fontmin.initWasm()
      const assets = await fontmin.optimizeBrowser({
        assets: [{ contents: ttf, fileName: 'roboto.ttf' }],
        plugins: fontmin.modernWeb({
          fontFamily: 'Roboto WASM',
          text: 'Hello Browser',
        }),
      })
      const woff2 = assets.find(asset => asset.fileName === 'roboto.woff2')
      const css = assets.find(asset => asset.fileName === 'roboto.css')
      const face = new FontFace('Roboto WASM', woff2.contents)
      document.fonts.add(face)
      await face.load()

      return {
        css: new TextDecoder().decode(css.contents),
        loaded: document.fonts.check("32px 'Roboto WASM'", 'Hello Browser'),
        woff2: new TextDecoder().decode(woff2.contents.subarray(0, 4)),
      }
    }, baseUrl)

    if (
      result.woff2 !== 'wOF2' ||
      !result.loaded ||
      !result.css.includes("font-family: 'Roboto WASM'")
    ) {
      throw new Error(
        `browser WASM verification failed: ${JSON.stringify(result)}`,
      )
    }
  } finally {
    await browser.close()
  }
} finally {
  await new Promise(resolveServer => {
    server.close(resolveServer)
  })
}
