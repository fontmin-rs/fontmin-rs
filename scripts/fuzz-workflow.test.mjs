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
  assert.match(workflow, /continue-on-error: true/u)
  assert.match(workflow, /if: steps\.fuzz\.outcome == 'failure'/u)
  assert.match(workflow, /run: exit 1/u)
  for (const target of [
    'parsers',
    'converters',
    'configuration',
    'output_naming',
    'public_api',
  ]) {
    assert.match(workflow, new RegExp(`- ${target}`, 'u'))
  }
  assert.match(workflow, /cargo fuzz run \$\{\{ matrix\.target \}\}/u)
  assert.match(workflow, /fuzz-artifacts-\$\{\{ matrix\.target \}\}/u)
})

test('promotes trusted fuzz failures through a reviewable pull request', () => {
  assert.match(workflow, /promote-regression:/u)
  assert.match(workflow, /github\.event_name != 'pull_request'/u)
  assert.match(workflow, /issues: write/u)
  assert.match(workflow, /pull-requests: write/u)
  assert.match(workflow, /cargo fuzz tmin "\$target"/u)
  assert.match(workflow, /-exact_artifact_path=/u)
  assert.match(workflow, /-max_total_time=120/u)
  assert.match(workflow, /scripts\/promote-fuzz-artifacts\.mjs/u)
  assert.match(workflow, /--target "\$target"/u)
  assert.match(workflow, /git switch -c "\$branch"/u)
  assert.match(workflow, /gh pr create/u)
  assert.match(workflow, /gh issue create/u)
  assert.match(workflow, /compare\/main\.\.\.\$\{branch\}\?expand=1/u)
})

test('preserves malformed fixtures as platform-independent bytes', () => {
  assert.match(attributes, /^fixtures\/malformed\/\*\.bin binary$/mu)
  assert.match(attributes, /^fixtures\/malformed\/\*\.hex binary$/mu)
})
