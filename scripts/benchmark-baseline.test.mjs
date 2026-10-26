import assert from 'node:assert/strict'
import test from 'node:test'
import {
  aggregateBenchmarkReports,
  normalizeBenchmarkReport,
} from './benchmark-baseline.mjs'

test('normalizes Vitest 5 JSON statistics for portable review', () => {
  const report = normalizeBenchmarkReport(
    {
      testResults: [
        {
          name: '/workspace/packages/fontmin/bench/subset.bench.ts',
          assertionResults: [
            {
              benchmarks: [
                {
                  name: 'subset',
                  tasks: [
                    {
                      name: 'subsetTtf text',
                      latency: {
                        mean: 12.2149603,
                        p75: 12.369208,
                        p99: 13.255083,
                        rme: 0.720542,
                        samplesCount: 41,
                      },
                      throughput: { mean: 81.966823 },
                    },
                  ],
                },
              ],
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
      hz: 81.9668,
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

test('keeps trial identities stable when Vitest reorders files and ranked tasks', () => {
  const files = ['subset', 'fontmin'].map(group => ({
    name: `/workspace/packages/fontmin/bench/${group}.bench.ts`,
    assertionResults: [
      {
        benchmarks: [
          {
            name: group,
            tasks: ['second', 'first'].map((name, index) => ({
              name,
              latency: {
                mean: index + 1,
                p75: index + 1,
                p99: index + 1,
                rme: 1,
                samplesCount: 10,
              },
              throughput: { mean: 1000 / (index + 1) },
            })),
          },
        ],
      },
    ],
  }))
  const reordered = files.toReversed().map(file => ({
    ...file,
    assertionResults: file.assertionResults.map(result => ({
      ...result,
      benchmarks: result.benchmarks.map(group => ({
        ...group,
        tasks: group.tasks.toReversed(),
      })),
    })),
  }))
  const metadata = { root: '/workspace' }
  const normalized = normalizeBenchmarkReport({ testResults: files }, metadata)

  assert.deepEqual(
    normalizeBenchmarkReport({ testResults: reordered }, metadata),
    normalized,
  )
  assert.equal(normalized.benchmarks[0].name, 'first')
  assert.equal(normalized.benchmarks[0].meanMs, 2)
  assert.equal(normalized.benchmarks[0].hz, 500)
})

test('aggregates like-for-like trials and verifies compatibility parity', () => {
  const report = {
    schemaVersion: 1,
    generatedAt: '2026-07-26T00:00:00.000Z',
    version: '0.1.0-beta.2',
    environment: { arch: 'arm64', bindingProfile: 'release', os: 'darwin' },
    fixture: { path: 'fixtures/fonts/ttf/roboto-regular.ttf' },
    benchmarks: [
      {
        file: 'packages/fontmin/bench/fontmin.bench.ts',
        group: 'fontmin baseline',
        hz: 500,
        meanMs: 2,
        name: 'fontmin-rs glyph + ttf2woff',
        p75Ms: 2.1,
        p99Ms: 2.4,
        rmePercent: 1,
        samples: 100,
      },
      {
        file: 'packages/fontmin/bench/fontmin.bench.ts',
        group: 'fontmin baseline',
        hz: 80,
        meanMs: 12.5,
        name: 'fontmin glyph + ttf2woff',
        p75Ms: 13,
        p99Ms: 15,
        rmePercent: 2,
        samples: 20,
      },
    ],
  }
  const aggregated = aggregateBenchmarkReports([
    report,
    {
      ...report,
      benchmarks: report.benchmarks.map((benchmark, index) => ({
        ...benchmark,
        meanMs: index === 0 ? 1.8 : 12,
      })),
    },
    {
      ...report,
      benchmarks: report.benchmarks.map((benchmark, index) => ({
        ...benchmark,
        meanMs: index === 0 ? 2.1 : 13,
      })),
    },
  ])

  assert.equal(aggregated.schemaVersion, 2)
  assert.equal(aggregated.trialCount, 3)
  assert.deepEqual(aggregated.benchmarks[0].trialMeanMs, [2, 1.8, 2.1])
  assert.equal(aggregated.benchmarks[0].meanMs, 2)
  assert.equal(aggregated.benchmarks[0].samples, 300)
  assert.deepEqual(aggregated.comparisons, [
    {
      name: 'compatibility glyph + ttf2woff parity',
      candidate: 'fontmin-rs glyph + ttf2woff',
      reference: 'fontmin glyph + ttf2woff',
      meanRatio: 0.16,
      maximumMeanRatio: 1.1,
      status: 'passed',
    },
  ])
})

test('rejects benchmark trials with different cases', () => {
  const first = {
    benchmarks: [
      {
        file: 'first.ts',
        group: 'first',
        name: 'fontmin-rs glyph + ttf2woff',
      },
    ],
  }
  const second = {
    benchmarks: [
      {
        file: 'second.ts',
        group: 'second',
        name: 'fontmin glyph + ttf2woff',
      },
    ],
  }

  assert.throws(
    () => aggregateBenchmarkReports([first, second]),
    /do not contain the same cases/u,
  )
})
