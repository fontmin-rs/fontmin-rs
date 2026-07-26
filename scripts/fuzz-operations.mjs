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

export function fuzzOperation(operationByte) {
  const id = operationByte % fuzzOperations.length

  return { id, name: fuzzOperations[id] }
}
