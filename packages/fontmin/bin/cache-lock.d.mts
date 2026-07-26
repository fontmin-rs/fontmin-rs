export function withCacheLock<T>(
  cacheRoot: string,
  operation: () => Promise<T>,
): Promise<T>
