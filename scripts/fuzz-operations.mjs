export const fuzzOperations = [
  'inspect',
  'analyzeCoverage',
  'subsetTtf',
  'woffRoundTrip',
  'woff2RoundTrip',
  'validateWoff2',
  'eotRoundTrip',
  'otfToTtf',
  'svgFontToTtf',
  'ttfToSvg',
  'ttfToWoff',
  'ttfToWoff2',
  'ttfToEot',
]

const operationsByTarget = {
  configuration: ['deserializeConfig'],
  converters: [
    'subsetTtf',
    'ttfToWoff',
    'ttfToWoff2',
    'ttfToEot',
    'otfToTtf',
    'svgFontToTtf',
    'ttfToSvg',
  ],
  output_naming: ['containedPath', 'renameExtension'],
  parsers: [
    'inspect',
    'analyzeCoverage',
    'woffToTtf',
    'woff2ToTtf',
    'validateWoff2',
    'eotToTtf',
    'otfToTtf',
  ],
  public_api: fuzzOperations,
}

export const focusedFuzzTargets = [
  'parsers',
  'converters',
  'configuration',
  'output_naming',
]

export const fuzzTargetNames = [...focusedFuzzTargets, 'public_api']

export function fuzzTargetOperations(target) {
  const operations = operationsByTarget[target]

  if (operations === undefined) {
    throw new Error(`unknown fuzz target: ${target}`)
  }

  return operations
}

export function fuzzOperation(operationByte, target = 'public_api') {
  const operations = fuzzTargetOperations(target)
  const id = operationByte % operations.length

  return { id, name: operations[id] }
}
