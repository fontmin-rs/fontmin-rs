export type DeliveryLanguagePreset =
  | 'ar'
  | 'el'
  | 'en'
  | 'hi'
  | 'ja'
  | 'ko'
  | 'ru'
  | 'zh-Hans'
  | 'zh-Hant'

export interface AutoDeliveryPlanOptions {
  frequencyText?: string
  languages?: DeliveryLanguagePreset[]
  maxSlices?: number
  targetBytes?: number
  tolerance?: number
}

export interface AutoDeliveryPlanSlice {
  codePoints: number[]
  estimatedBytes: number
  name: string
  unicodeRanges: string[]
}

export interface AutoDeliveryPlan {
  codePointCount: number
  languages: DeliveryLanguagePreset[]
  slices: AutoDeliveryPlanSlice[]
  targetBytes: number
  tolerance: number
}

interface CodePointRange {
  end: number
  start: number
}

interface PlanningGroup {
  codePoints: number[]
  name: string
}

interface MeasuredGroup extends PlanningGroup {
  estimatedBytes: number
}

interface LanguageGroup {
  name: string
  ranges: CodePointRange[]
}

export type MeasureDeliverySlice = (
  codePoints: readonly number[],
) => Promise<number>

const DEFAULT_TARGET_BYTES = 100 * 1024
const DEFAULT_TOLERANCE = 0.15
const DEFAULT_MAX_SLICES = 32

/* oxlint-disable unicorn/numeric-separators-style -- Unicode block endpoints follow standard hexadecimal notation. */
const GROUPS = {
  arabic: ranges([
    [0x0600, 0x06ff],
    [0x0750, 0x077f],
    [0x0870, 0x089f],
    [0x08a0, 0x08ff],
    [0xfb50, 0xfdff],
    [0xfe70, 0xfeff],
  ]),
  bopomofo: ranges([
    [0x3100, 0x312f],
    [0x31a0, 0x31bf],
  ]),
  cyrillic: ranges([
    [0x0400, 0x052f],
    [0x1c80, 0x1c8f],
    [0x2de0, 0x2dff],
    [0xa640, 0xa69f],
  ]),
  devanagari: ranges([
    [0x0900, 0x097f],
    [0xa8e0, 0xa8ff],
  ]),
  greek: ranges([
    [0x0370, 0x03ff],
    [0x1f00, 0x1fff],
  ]),
  han: ranges([
    [0x2e80, 0x2fdf],
    [0x3400, 0x4dbf],
    [0x4e00, 0x9fff],
    [0xf900, 0xfaff],
    [0x20_000, 0x2e_bef],
    [0x30_000, 0x32_3af],
  ]),
  hangul: ranges([
    [0x1100, 0x11ff],
    [0x3130, 0x318f],
    [0xa960, 0xa97f],
    [0xac00, 0xd7af],
    [0xd7b0, 0xd7ff],
  ]),
  kana: ranges([
    [0x3040, 0x30ff],
    [0x31f0, 0x31ff],
    [0x1b_000, 0x1b_16f],
  ]),
  latin: ranges([
    [0x0020, 0x007e],
    [0x00a0, 0x02af],
    [0x1e00, 0x1eff],
  ]),
  punctuation: ranges([
    [0x2000, 0x206f],
    [0x3000, 0x303f],
    [0xfe10, 0xfe1f],
    [0xfe30, 0xfe4f],
    [0xff00, 0xffef],
  ]),
} satisfies Record<string, CodePointRange[]>
/* oxlint-enable unicorn/numeric-separators-style */

const LANGUAGE_GROUPS: Record<DeliveryLanguagePreset, string[]> = {
  ar: ['arabic'],
  el: ['greek'],
  en: ['latin'],
  hi: ['devanagari'],
  ja: ['punctuation', 'kana', 'han'],
  ko: ['punctuation', 'hangul', 'han'],
  ru: ['cyrillic'],
  'zh-Hans': ['punctuation', 'han'],
  'zh-Hant': ['punctuation', 'bopomofo', 'han'],
}

/**
 * Detect supported delivery-language presets from representative page text.
 *
 * @param text - Static or frequency-weighted business text.
 * @returns Presets in first-observed order, with Han assigned to a detected
 *   Japanese, Korean, or Traditional Chinese context before Simplified Chinese.
 */
export function detectDeliveryLanguages(
  text: string,
): DeliveryLanguagePreset[] {
  const detected: DeliveryLanguagePreset[] = []
  const seen = new Set<DeliveryLanguagePreset>()
  let pendingHan = false

  for (const character of text) {
    const codePoint = character.codePointAt(0)
    if (codePoint === undefined) {
      continue
    }
    const language = languageOf(codePoint)

    if (language === 'han') {
      pendingHan = true
    } else if (language !== undefined && !seen.has(language)) {
      seen.add(language)
      detected.push(language)
    }
  }

  const contextualHanLanguages: DeliveryLanguagePreset[] = [
    'ja',
    'ko',
    'zh-Hant',
  ]
  if (pendingHan && !contextualHanLanguages.some(tag => seen.has(tag))) {
    detected.push('zh-Hans')
  }

  return detected
}

/**
 * Plan byte-bounded delivery slices over code points supported by a font.
 *
 * @param supportedCodePoints - Unicode cmap coverage from one or more faces.
 * @param options - Language, frequency, byte target, and request constraints.
 * @param measure - Returns the actual encoded byte size for a proposed slice.
 * @returns A deterministic, measured delivery plan.
 */
export async function planAutoDeliverySlices(
  supportedCodePoints: readonly number[],
  options: AutoDeliveryPlanOptions,
  measure: MeasureDeliverySlice,
): Promise<AutoDeliveryPlan> {
  const normalized = normalizeOptions(options)
  const languages = resolveLanguages(normalized)
  const groups = planningGroups(supportedCodePoints, languages, normalized)

  if (groups.length === 0) {
    throw new Error('auto delivery presets matched no supported code points')
  }
  if (groups.length > normalized.maxSlices) {
    throw new Error(
      `auto delivery requires at least ${groups.length} slices for the selected languages`,
    )
  }

  const measured = await Promise.all(
    groups.map(async group => measureGroup(group, measure)),
  )
  const split = await splitOversizedGroups(measured, normalized, measure)
  const merged = await mergeSmallGroups(split, normalized, measure)
  const groupCounts = new Map<string, number>()

  for (const group of merged) {
    groupCounts.set(group.name, (groupCounts.get(group.name) ?? 0) + 1)
  }
  const groupIndexes = new Map<string, number>()
  const slices = merged.map(group => {
    const index = (groupIndexes.get(group.name) ?? 0) + 1
    const count = groupCounts.get(group.name) ?? 1
    groupIndexes.set(group.name, index)

    return {
      codePoints: [...group.codePoints].toSorted((left, right) => left - right),
      estimatedBytes: group.estimatedBytes,
      name:
        count === 1
          ? group.name
          : `${group.name}-${String(index).padStart(String(count).length, '0')}`,
      unicodeRanges: unicodeRangesFromCodePoints(group.codePoints),
    }
  })

  return {
    codePointCount: new Set(slices.flatMap(slice => slice.codePoints)).size,
    languages,
    slices,
    targetBytes: normalized.targetBytes,
    tolerance: normalized.tolerance,
  }
}

function normalizeOptions(
  options: AutoDeliveryPlanOptions,
): Required<AutoDeliveryPlanOptions> {
  const targetBytes = options.targetBytes ?? DEFAULT_TARGET_BYTES
  const tolerance = options.tolerance ?? DEFAULT_TOLERANCE
  const maxSlices = options.maxSlices ?? DEFAULT_MAX_SLICES

  if (!Number.isInteger(targetBytes) || targetBytes <= 0) {
    throw new TypeError('auto delivery targetBytes must be a positive integer')
  }
  if (!Number.isFinite(tolerance) || tolerance < 0 || tolerance >= 1) {
    throw new TypeError('auto delivery tolerance must be in [0, 1)')
  }
  if (!Number.isInteger(maxSlices) || maxSlices <= 0 || maxSlices > 256) {
    throw new TypeError(
      'auto delivery maxSlices must be an integer in [1, 256]',
    )
  }

  return {
    frequencyText: options.frequencyText ?? '',
    languages: [...(options.languages ?? [])],
    maxSlices,
    targetBytes,
    tolerance,
  }
}

function resolveLanguages(
  options: Required<AutoDeliveryPlanOptions>,
): DeliveryLanguagePreset[] {
  const languages =
    options.languages.length === 0
      ? detectDeliveryLanguages(options.frequencyText)
      : options.languages
  const resolved = languages.length === 0 ? ['en' as const] : languages

  return [...new Set(resolved)]
}

function planningGroups(
  supportedCodePoints: readonly number[],
  languages: DeliveryLanguagePreset[],
  options: Required<AutoDeliveryPlanOptions>,
): PlanningGroup[] {
  const supported = new Set(
    supportedCodePoints
      .filter(codePoint => isValidUnicodeScalar(codePoint))
      .toSorted((left, right) => left - right),
  )
  const languageGroups = groupsForLanguages(languages)
  const selected = new Set(
    languageGroups.flatMap(group =>
      [...supported].filter(codePoint => includesCodePoint(group, codePoint)),
    ),
  )
  const frequency = frequencyOrder(options.frequencyText).filter(codePoint =>
    selected.has(codePoint),
  )
  const prioritized = new Set(frequency)
  const groups: PlanningGroup[] = []

  if (frequency.length > 0) {
    groups.push({ codePoints: frequency, name: 'priority' })
  }
  const assigned = new Set<number>(prioritized)
  for (const group of languageGroups) {
    const codePoints = [...supported].filter(
      codePoint =>
        !assigned.has(codePoint) && includesCodePoint(group, codePoint),
    )
    for (const codePoint of codePoints) {
      assigned.add(codePoint)
    }
    if (codePoints.length > 0) {
      groups.push({ codePoints, name: group.name })
    }
  }

  return groups
}

function groupsForLanguages(
  languages: DeliveryLanguagePreset[],
): LanguageGroup[] {
  const names = [
    ...new Set(languages.flatMap(language => LANGUAGE_GROUPS[language])),
  ]

  return names.map(name => ({
    name,
    ranges: GROUPS[name as keyof typeof GROUPS],
  }))
}

async function splitOversizedGroups(
  initial: MeasuredGroup[],
  options: Required<AutoDeliveryPlanOptions>,
  measure: MeasureDeliverySlice,
): Promise<MeasuredGroup[]> {
  const groups = [...initial]
  const maximumBytes = options.targetBytes * (1 + options.tolerance)

  while (groups.length < options.maxSlices) {
    const candidateIndex = groups
      .map((group, index) => ({ group, index }))
      .filter(
        ({ group }) =>
          group.estimatedBytes > maximumBytes && group.codePoints.length > 1,
      )
      .toSorted(
        (left, right) =>
          right.group.estimatedBytes - left.group.estimatedBytes ||
          left.index - right.index,
      )[0]?.index

    if (candidateIndex === undefined) {
      break
    }
    const candidate = groups[candidateIndex]
    if (candidate === undefined) {
      break
    }
    const midpoint = Math.ceil(candidate.codePoints.length / 2)
    const replacements = await Promise.all([
      measureGroup(
        {
          codePoints: candidate.codePoints.slice(0, midpoint),
          name: candidate.name,
        },
        measure,
      ),
      measureGroup(
        {
          codePoints: candidate.codePoints.slice(midpoint),
          name: candidate.name,
        },
        measure,
      ),
    ])
    groups.splice(candidateIndex, 1, ...replacements)
  }

  return groups
}

async function mergeSmallGroups(
  initial: MeasuredGroup[],
  options: Required<AutoDeliveryPlanOptions>,
  measure: MeasureDeliverySlice,
): Promise<MeasuredGroup[]> {
  const groups = [...initial]
  const minimumBytes = options.targetBytes * (1 - options.tolerance)
  const maximumBytes = options.targetBytes * (1 + options.tolerance)
  let index = 0

  while (index < groups.length - 1) {
    const current = groups[index]
    const next = groups[index + 1]
    if (current === undefined || next === undefined) {
      break
    }

    if (current.name !== next.name || current.estimatedBytes >= minimumBytes) {
      index += 1
      continue
    }
    const merged = await measureGroup(
      {
        codePoints: [...current.codePoints, ...next.codePoints],
        name: current.name,
      },
      measure,
    )
    if (merged.estimatedBytes <= maximumBytes) {
      groups.splice(index, 2, merged)
    } else {
      index += 1
    }
  }

  return groups
}

async function measureGroup(
  group: PlanningGroup,
  measure: MeasureDeliverySlice,
): Promise<MeasuredGroup> {
  const estimatedBytes = await measure(group.codePoints)

  if (!Number.isInteger(estimatedBytes) || estimatedBytes < 0) {
    throw new TypeError(
      'auto delivery measure must return a non-negative integer',
    )
  }

  return { ...group, estimatedBytes }
}

function frequencyOrder(text: string): number[] {
  const frequencies = new Map<number, { count: number; index: number }>()
  let index = 0

  for (const character of text) {
    const codePoint = character.codePointAt(0)
    if (codePoint !== undefined) {
      const current = frequencies.get(codePoint)
      frequencies.set(codePoint, {
        count: (current?.count ?? 0) + 1,
        index: current?.index ?? index,
      })
      index += 1
    }
  }

  return [...frequencies.entries()]
    .toSorted(
      ([, left], [, right]) =>
        right.count - left.count || left.index - right.index,
    )
    .map(([codePoint]) => codePoint)
}

/**
 * Compact code points into canonical CSS Unicode ranges.
 *
 * @param codePoints - Unicode scalar values in any order.
 * @returns Sorted `U+XXXX[-YYYY]` ranges.
 */
export function unicodeRangesFromCodePoints(
  codePoints: readonly number[],
): string[] {
  const sorted = [...new Set(codePoints)].toSorted(
    (left, right) => left - right,
  )
  const compact: string[] = []

  for (let index = 0; index < sorted.length;) {
    const start = sorted[index]
    if (start === undefined) {
      break
    }
    let end = start

    while (sorted[index + 1] === end + 1) {
      end += 1
      index += 1
    }
    compact.push(
      start === end
        ? `U+${unicodeHex(start)}`
        : `U+${unicodeHex(start)}-${unicodeHex(end)}`,
    )
    index += 1
  }

  return compact
}

function languageOf(
  codePoint: number,
): DeliveryLanguagePreset | 'han' | undefined {
  if (includesRanges(GROUPS.kana, codePoint)) {
    return 'ja'
  }
  if (includesRanges(GROUPS.hangul, codePoint)) {
    return 'ko'
  }
  if (includesRanges(GROUPS.bopomofo, codePoint)) {
    return 'zh-Hant'
  }
  if (includesRanges(GROUPS.han, codePoint)) {
    return 'han'
  }
  if (includesRanges(GROUPS.arabic, codePoint)) {
    return 'ar'
  }
  if (includesRanges(GROUPS.devanagari, codePoint)) {
    return 'hi'
  }
  if (includesRanges(GROUPS.cyrillic, codePoint)) {
    return 'ru'
  }
  if (includesRanges(GROUPS.greek, codePoint)) {
    return 'el'
  }
  if (includesRanges(GROUPS.latin, codePoint)) {
    return 'en'
  }

  return undefined
}

function includesCodePoint(group: LanguageGroup, codePoint: number): boolean {
  return includesRanges(group.ranges, codePoint)
}

function includesRanges(
  values: readonly CodePointRange[],
  codePoint: number,
): boolean {
  return values.some(
    range => codePoint >= range.start && codePoint <= range.end,
  )
}

function ranges(values: [number, number][]): CodePointRange[] {
  return values.map(([start, end]) => ({ end, start }))
}

function isValidUnicodeScalar(codePoint: number): boolean {
  return (
    Number.isInteger(codePoint) &&
    codePoint >= 0 &&
    codePoint <= 0x10_ff_ff &&
    (codePoint < 0xd8_00 || codePoint > 0xdf_ff)
  )
}

function unicodeHex(codePoint: number): string {
  return codePoint.toString(16).toUpperCase().padStart(4, '0')
}
