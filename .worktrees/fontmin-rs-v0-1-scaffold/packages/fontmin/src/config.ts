import type { FontminConfig } from './types'

export function defineConfig<T extends FontminConfig>(config: T): T {
  return config
}
