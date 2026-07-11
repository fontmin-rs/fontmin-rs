import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { chromium } from 'playwright'
import { css, glyph, optimize, ttf2woff, ttf2woff2 } from '../dist/index.mjs'

const currentDir = import.meta.dirname
const fixture = resolve(
  currentDir,
  '../../../fixtures/fonts/ttf/roboto-regular.ttf',
)
const outDir = await mkBrowserLoadDir()
const fontFamily = 'Roboto Browser Load'
const sampleText = 'Hello Browser'

try {
  await optimize({
    input: [fixture],
    outDir,
    plugins: [
      glyph({ text: sampleText }),
      ttf2woff2(),
      ttf2woff(),
      css({
        fontDisplay: 'swap',
        fontFamily,
        fontPath: './',
        local: false,
      }),
    ],
  })

  await assertHeader(join(outDir, 'roboto-regular.woff2'), 'wOF2')
  await assertHeader(join(outDir, 'roboto-regular.woff'), 'wOFF')

  const htmlPath = join(outDir, 'index.html')
  await writeFile(
    htmlPath,
    `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <link rel="stylesheet" href="./roboto-regular.css">
    <style>
      #sample {
        font-family: '${fontFamily}', monospace;
        font-size: 32px;
        line-height: 1.4;
      }
    </style>
  </head>
  <body>
    <p id="sample">${sampleText}</p>
  </body>
</html>
`,
  )

  const browser = await chromium.launch({ headless: true })
  try {
    const page = await browser.newPage()
    await page.goto(pathToFileURL(htmlPath).href)

    const result = await page.evaluate(
      async ({ fontFamily, sampleText }) => {
        await document.fonts.ready
        const loaded = await document.fonts.load(
          `32px '${fontFamily}'`,
          sampleText,
        )
        const sample = document.querySelector('#sample')

        return {
          check: document.fonts.check(`32px '${fontFamily}'`, sampleText),
          computedFamily: sample ? getComputedStyle(sample).fontFamily : '',
          loaded: loaded.map(font => ({
            family: font.family,
            status: font.status,
          })),
          text: sample?.textContent,
          width: sample?.getBoundingClientRect().width ?? 0,
        }
      },
      { fontFamily, sampleText },
    )

    if (!result.check) {
      throw new Error(`browser did not report ${fontFamily} as loaded`)
    }
    if (!result.computedFamily.includes(fontFamily)) {
      throw new Error(
        `expected computed font-family to include ${fontFamily}, got ${result.computedFamily}`,
      )
    }
    if (!result.loaded.some(font => font.family === fontFamily)) {
      throw new Error(
        `document.fonts.load did not return ${fontFamily}: ${JSON.stringify(result.loaded)}`,
      )
    }
    if (result.text !== sampleText || result.width <= 0) {
      throw new Error(
        `browser rendered unexpected sample: ${JSON.stringify(result)}`,
      )
    }
  } finally {
    await browser.close()
  }
} finally {
  await rm(outDir, { force: true, recursive: true })
}

async function assertHeader(path, expected) {
  const bytes = await readFile(path)
  const header = bytes.subarray(0, 4).toString('ascii')

  if (header !== expected) {
    throw new Error(`expected ${path} to start with ${expected}, got ${header}`)
  }
}

async function mkBrowserLoadDir() {
  return mkdtemp(join(tmpdir(), 'fontmin-rs-browser-load-'))
}
