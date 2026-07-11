import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)
const binding = require('./bindings.js')

export const { subsetTtf } = binding
