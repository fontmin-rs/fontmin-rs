export class ByteLruCache<Key> {
  readonly #entries = new Map<Key, Uint8Array>()
  readonly #maxBytes: number
  readonly #maxEntries: number
  #bytes = 0

  constructor(options: { maxBytes: number; maxEntries: number }) {
    this.#maxBytes = options.maxBytes
    this.#maxEntries = options.maxEntries
  }

  get(key: Key): Uint8Array | undefined {
    const value = this.#entries.get(key)

    if (value !== undefined) {
      this.#entries.delete(key)
      this.#entries.set(key, value)
    }

    return value
  }

  set(key: Key, value: Uint8Array): void {
    const existing = this.#entries.get(key)
    if (existing !== undefined) {
      this.#bytes -= existing.byteLength
      this.#entries.delete(key)
    }
    if (value.byteLength > this.#maxBytes) {
      return
    }

    this.#entries.set(key, value)
    this.#bytes += value.byteLength

    while (
      this.#bytes > this.#maxBytes ||
      this.#entries.size > this.#maxEntries
    ) {
      const oldest = this.#entries.entries().next().value
      if (oldest === undefined) {
        break
      }
      const [oldestKey, oldestValue] = oldest

      this.#entries.delete(oldestKey)
      this.#bytes -= oldestValue.byteLength
    }
  }
}
