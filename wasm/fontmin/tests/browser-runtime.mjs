import assert from 'node:assert/strict'
import { execFile } from 'node:child_process'
import {
  copyFile,
  mkdtemp,
  readFile,
  readdir,
  rm,
  writeFile,
} from 'node:fs/promises'
import { createServer } from 'node:http'
import { tmpdir } from 'node:os'
import { extname, join, resolve } from 'node:path'
import { promisify } from 'node:util'
import { chromium, firefox, webkit } from 'playwright'

const executeFile = promisify(execFile)
const browserName = process.env.BROWSER ?? 'chromium'
const launcher = { chromium, firefox, webkit }[browserName]
const workspaceRoot = resolve(import.meta.dirname, '../../..')
const consumerRoot = await mkdtemp(join(tmpdir(), 'fontmin-wasm-browser-'))
const tarballRoot = join(consumerRoot, 'tarballs')
let server

try {
  await writeFile(
    join(consumerRoot, 'package.json'),
    JSON.stringify({
      name: 'fontmin-wasm-browser-smoke',
      private: true,
      type: 'module',
    }),
  )
  await copyFile(
    join(workspaceRoot, 'fixtures/fonts/ttf/roboto-regular.ttf'),
    join(consumerRoot, 'roboto.ttf'),
  )
  await executeFile('pnpm', ['pack', '--pack-destination', tarballRoot], {
    cwd: join(workspaceRoot, 'wasm/fontmin'),
  })
  const tarballEntries = await readdir(tarballRoot)
  const tarballs = tarballEntries.filter(fileName => fileName.endsWith('.tgz'))
  assert.deepEqual(tarballs.length, 1)
  await executeFile(
    'npm',
    ['install', '--ignore-scripts', join(tarballRoot, tarballs[0])],
    { cwd: consumerRoot },
  )

  server = createServer(async (request, response) => {
    const pathname = new URL(request.url ?? '/', 'http://localhost').pathname
    if (pathname === '/') {
      response
        .writeHead(200, { 'content-type': 'text/html' })
        .end('<!doctype html>')
      return
    }
    const path = resolve(consumerRoot, `.${pathname}`)

    if (!path.startsWith(consumerRoot)) {
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
  const browser = await launcher.launch({ headless: true })

  try {
    const page = await browser.newPage()
    await page.goto(baseUrl)
    const result = await page.evaluate(async base => {
      const fontmin = await import(
        `${base}/node_modules/@fontmin-rs/wasm/dist/index.mjs`
      )
      const response = await fetch(`${base}/roboto.ttf`)
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
        `packed browser WASM verification failed: ${JSON.stringify(result)}`,
      )
    }
  } finally {
    await browser.close()
  }
} finally {
  if (server !== undefined) {
    await new Promise(resolveServer => {
      server.close(resolveServer)
    })
  }
  await rm(consumerRoot, { force: true, recursive: true })
}
