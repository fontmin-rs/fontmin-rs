import assert from 'node:assert/strict'
import test from 'node:test'
import { normalizeBenchmarkReport } from './benchmark-baseline.mjs'

test('normalizes Vitest benchmark output for portable review', () => {
  const report = normalizeBenchmarkReport(
    {
      files: [
        {
          filepath: '/workspace/packages/fontmin/bench/subset.bench.ts',
          groups: [
            {
              benchmarks: [
                {
                  hz: 81.866823,
                  mean: 12.2149603,
                  name: 'subsetTtf text',
                  p75: 12.369208,
                  p99: 13.255083,
                  rme: 0.720542,
                  sampleCount: 41,
                },
              ],
              fullName: 'bench/subset.bench.ts > subset',
            },
          ],
        },
      ],
    },
    {
      environment: { arch: 'arm64', os: 'darwin' },
      fixture: { path: 'fixtures/fonts/ttf/roboto-regular.ttf' },
      generatedAt: '2026-07-21T00:00:00.000Z',
      root: '/workspace',
      version: '0.1.0-beta.2',
    },
  )

  assert.deepEqual(report.benchmarks, [
    {
      file: 'packages/fontmin/bench/subset.bench.ts',
      group: 'bench/subset.bench.ts > subset',
      hz: 81.8668,
      meanMs: 12.215,
      name: 'subsetTtf text',
      p75Ms: 12.3692,
      p99Ms: 13.2551,
      rmePercent: 0.7205,
      samples: 41,
    },
  ])
  assert.equal(report.schemaVersion, 1)
  assert.equal(report.version, '0.1.0-beta.2')
})
