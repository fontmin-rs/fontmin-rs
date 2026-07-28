# Cache Semantics

The Rust CLI and Node.js pipeline use the same default cache directory,
`node_modules/.cache/fontmin-rs`, but maintain their own cache entries and
runtime identities. Caching is opt-in. See [Configuration](./guide/config) for
the public options.

## Consistency

Each versioned cache root permits one writer at a time. Writers create an
exclusive owner-tagged lock, write files through temporary paths, and
atomically replace completed files. Concurrent processes therefore serialize
cache updates rather than modifying the index or one entry simultaneously.

Lock release is owner-aware: a writer only removes the lock whose owner token
it acquired. This prevents a delayed cleanup from removing a successor's lock.
Readers treat an incomplete or missing entry as a cache miss and rebuild it.

## Process termination and cancellation

Normal completion and errors release the lock. Cancelling an in-process Rust
cache write also releases its lock when the write task is dropped.

When a process terminates while writing, the next writer detects that the
recorded process is no longer running and reclaims the lock. A five-minute age
limit remains as a fallback for legacy or unreadable owner identifiers. While
recovering, the writer removes temporary `*.tmp` files from the versioned cache
tree before starting a new update. Rust task cancellation performs the same
cleanup while releasing its lock.

The acquisition retry window is approximately five seconds. A timeout means
another live writer still owns the cache, or the cache is on a filesystem
where local process ownership cannot be verified.

## Operational boundary

The lock protocol is designed for independent processes on one host. Do not
share one writable cache directory between different machines over a network
filesystem, because process identifiers are host-local. Give each machine its
own cache directory and share only immutable build artifacts.
