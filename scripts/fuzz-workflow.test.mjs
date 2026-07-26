import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import test from 'node:test'

const [attributes, workflow] = await Promise.all([
  readFile('.gitattributes', 'utf8'),
  readFile('.github/workflows/fuzz.yml', 'utf8'),
])

test('runs bounded AddressSanitizer fuzzing on changes and a schedule', () => {
  assert.match(workflow, /schedule:/u)
  assert.match(workflow, /workflow_dispatch:/u)
  assert.match(workflow, /tool: cargo-fuzz/u)
  assert.match(workflow, /--target x86_64-unknown-linux-gnu/u)
  assert.match(workflow, /--sanitizer address/u)
  assert.match(workflow, /-max_total_time=\$\{FUZZ_SECONDS\}/u)
  assert.match(workflow, /-max_len=1048576/u)
  assert.match(workflow, /-timeout=10/u)
  assert.match(workflow, /if: failure\(\)/u)
})

test('preserves malformed fixtures as platform-independent bytes', () => {
  assert.match(attributes, /^fixtures\/malformed\/\*\.bin binary$/mu)
})
