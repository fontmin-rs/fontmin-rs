# Text corpus

These UTF-8 samples keep coverage, subsetting, and benchmark scenarios named
and repeatable without duplicating strings across test suites.

- `latin.txt` exercises ASCII, accents, punctuation, and digits.
- `cjk.txt` exercises common Simplified Chinese, Traditional Chinese,
  Japanese, and Korean text. It is also useful for missing-glyph audits until
  the binary corpus gains a CJK fixture.
- `symbols.txt` exercises common technical, currency, mathematical, and arrow
  symbols.

Benchmarks should identify both the font fixture and text corpus they use.
Correctness tests may intentionally expect missing code points when a corpus
extends beyond a font's declared coverage.
