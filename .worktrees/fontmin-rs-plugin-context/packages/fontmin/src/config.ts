import { readFile, stat } from 'node:fs/promises'
import { dirname, extname, resolve } from 'node:path'
import { pathToFileURL } from 'node:url'
import { parse as parseJsonc } from 'jsonc-parser'
import type { FontminConfig } from './types'

const DEFAULT_CONFIG_FILES = [
  'fontmin.config.ts',
  'fontmin.config.mts',
  'fontmin.config.mjs',
  'fontmin.config.cjs',
  'fontmin.config.json',
  'fontmin.config.jsonc',
]

export function defineConfig<T extends FontminConfig>(config: T): T {
  return config
}

export async function loadConfig(configPath?: string): Promise<FontminConfig> {
  const resolvedPath =
    configPath === undefined ? await findConfig() : resolve(configPath)
  const extension = extname(resolvedPath)
  const configDir = dirname(resolvedPath)
  let config: FontminConfig

  if (extension === '.json') {
    const contents = await readFile(resolvedPath, 'utf8')
    config = JSON.parse(contents) as FontminConfig
  } else if (extension === '.jsonc') {
    const contents = await readFile(resolvedPath, 'utf8')
    config = parseJsonc(contents) as FontminConfig
  } else {
    const configModule = (await import(
      pathToFileURL(resolvedPath).href
    )) as ConfigModule
    const loadedConfig = configModule.default ?? configModule.config

    config =
      typeof loadedConfig === 'function'
        ? ((await loadedConfig()) as FontminConfig)
        : (loadedConfig as FontminConfig)
  }

  return defineConfig(withConfigDefaults(config, configDir))
}

export async function findConfig(cwd = process.cwd()): Promise<string> {
  for (const fileName of DEFAULT_CONFIG_FILES) {
    const configPath = resolve(cwd, fileName)

    if (await isFile(configPath)) {
      return configPath
    }
  }

  throw new Error(`could not find fontmin config in ${cwd}`)
}

async function isFile(path: string): Promise<boolean> {
  try {
    return (await stat(path)).isFile()
  } catch {
    return false
  }
}

function withConfigDefaults(
  config: FontminConfig,
  configDir: string,
): FontminConfig {
  if (config.cwd !== undefined) {
    return config
  }

  return {
    ...config,
    cwd: configDir,
  }
}

interface ConfigModule {
  default?: FontminConfig | (() => FontminConfig | Promise<FontminConfig>)
  config?: FontminConfig | (() => FontminConfig | Promise<FontminConfig>)
}
