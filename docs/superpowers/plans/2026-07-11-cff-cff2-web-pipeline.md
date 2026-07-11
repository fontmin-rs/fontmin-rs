# CFF/CFF2 Web Pipeline Implementation Plan

> Execute in order with focused tests after every behavior change.

1. Add `OtfConfig` to the Rust configuration model and map it into a
   pre-ordered, replacement-mode `Otf2TtfPlugin` in `fontmin_pipeline`.
2. Add repeated `build --variation TAG=VALUE` parsing, merge it with config
   coordinates, and cover static CFF plus selected CFF2 build output in CLI
   integration tests.
3. Prepend replacement-mode `otf2ttf()` to Node and browser `modernWeb()`;
   expose the OTF options in Node's public type and teach the browser optimizer
   to replace an OTF when `clone: false`.
4. Document direct CFF/CFF2 use through the Web preset in English and Chinese.
5. Run targeted Rust, Node, WASM, docs, and workspace verification. Do not
   publish, version, push, or alter release configuration.
