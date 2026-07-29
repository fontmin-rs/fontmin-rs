import type { FontminPlugin } from './types'

export interface BuiltinPluginDescriptor {
  kind: 'builtin'
  name: string
  options: Record<string, unknown>
}

export type BuiltinPlugin = FontminPlugin & {
  native: BuiltinPluginDescriptor
}

const INTERNAL_CACHE_KEY = Symbol('fontmin.internalCacheKey')

type InternallyCacheablePlugin = FontminPlugin & {
  [INTERNAL_CACHE_KEY]: string
}

export function createBuiltinPlugin(
  name: string,
  options: Record<string, unknown>,
  pluginName = `fontmin:${name}`,
): BuiltinPlugin {
  return {
    name: pluginName,
    native: {
      kind: 'builtin',
      name,
      options,
    },
  }
}

export function builtinPluginDescriptor(
  plugin: FontminPlugin,
  expectedName?: string,
): BuiltinPluginDescriptor | undefined {
  const descriptor: unknown = plugin.native

  if (
    typeof descriptor !== 'object' ||
    descriptor === null ||
    Array.isArray(descriptor) ||
    !('kind' in descriptor) ||
    descriptor.kind !== 'builtin'
  ) {
    return undefined
  }
  if (
    !('name' in descriptor) ||
    typeof descriptor.name !== 'string' ||
    descriptor.name.length === 0
  ) {
    throw new TypeError(
      `built-in plugin ${plugin.name} must provide a non-empty name`,
    )
  }
  if (
    !('options' in descriptor) ||
    typeof descriptor.options !== 'object' ||
    descriptor.options === null ||
    Array.isArray(descriptor.options)
  ) {
    throw new TypeError(
      `built-in plugin ${plugin.name} must provide an options object`,
    )
  }
  if (expectedName !== undefined && descriptor.name !== expectedName) {
    return undefined
  }

  return descriptor as BuiltinPluginDescriptor
}

export function isBuiltinPlugin(
  plugin: FontminPlugin,
  name: string,
): plugin is BuiltinPlugin {
  return builtinPluginDescriptor(plugin, name) !== undefined
}

export function withBuiltinPluginOptions(
  plugin: FontminPlugin,
  name: string,
  options: Record<string, unknown>,
): BuiltinPlugin {
  const descriptor = builtinPluginDescriptor(plugin, name)

  if (descriptor === undefined) {
    throw new TypeError(`plugin ${plugin.name} is not built-in ${name}`)
  }

  return {
    ...plugin,
    native: {
      ...descriptor,
      options,
    },
  }
}

export function withInternalCacheKey<T extends FontminPlugin>(
  plugin: T,
  key: string,
): T {
  return Object.assign(plugin, { [INTERNAL_CACHE_KEY]: key })
}

export function internalCacheKey(plugin: FontminPlugin): string | undefined {
  return (plugin as Partial<InternallyCacheablePlugin>)[INTERNAL_CACHE_KEY]
}

export function pluginUsesRuntime(plugin: FontminPlugin): boolean {
  return builtinPluginDescriptor(plugin) !== undefined
}

export function isCacheablePlugin(plugin: FontminPlugin): boolean {
  if (internalCacheKey(plugin) !== undefined) {
    return true
  }

  return (
    builtinPluginDescriptor(plugin) !== undefined &&
    plugin.buildStart === undefined &&
    plugin.transform === undefined &&
    plugin.generateBundle === undefined &&
    plugin.buildEnd === undefined
  )
}
