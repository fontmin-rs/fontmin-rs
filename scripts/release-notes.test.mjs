import assert from 'node:assert/strict'
import test from 'node:test'
import { extractReleaseNotes } from './release-notes.mjs'

const changelog = `# Changelog

## [Unreleased]

## [1.2.0-rc.1] - 2026-08-12

### Added

- First candidate feature.

### Fixed

- Candidate regression.

## [1.1.0] - 2026-08-01

### Added

- Previous feature.

[Unreleased]: https://example.com/compare/v1.2.0-rc.1...HEAD
[1.2.0-rc.1]: https://example.com/compare/v1.1.0...v1.2.0-rc.1
`

test('extracts the complete changelog section for a release', () => {
  assert.equal(
    extractReleaseNotes(changelog, '1.2.0-rc.1'),
    `### Added

- First candidate feature.

### Fixed

- Candidate regression.`,
  )
})

test('rejects a release without a changelog section', () => {
  assert.throws(
    () => extractReleaseNotes(changelog, '1.2.0'),
    /CHANGELOG\.md does not contain release notes for 1\.2\.0/u,
  )
})

test('rejects an empty release section', () => {
  assert.throws(
    () =>
      extractReleaseNotes(
        '# Changelog\n\n## [1.2.0] - 2026-08-12\n\n## [1.1.0] - 2026-08-01\n',
        '1.2.0',
      ),
    /release notes for 1\.2\.0 are empty/u,
  )
})

test('excludes comparison links from the oldest release notes', () => {
  assert.equal(
    extractReleaseNotes(
      `# Changelog

## [1.0.0] - 2026-08-01

### Added

- Initial feature.

[1.0.0]: https://example.com/releases/v1.0.0
`,
      '1.0.0',
    ),
    `### Added

- Initial feature.`,
  )
})
