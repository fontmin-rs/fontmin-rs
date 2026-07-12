# Rust CLI Module Configuration Design

## Goal

Bring the Rust CLI's configuration discovery and loading behavior in line with
the Node package for `.ts`, `.mts`, `.mjs`, and `.cjs` configuration modules,
without embedding a JavaScript engine or moving the Rust build pipeline into
the Node CLI.

## Supported Configuration Forms

The Rust CLI discovers configuration files in the same order as the Node
package:

1. `fontmin.config.ts`
2. `fontmin.config.mts`
3. `fontmin.config.mjs`
4. `fontmin.config.cjs`
5. `fontmin.config.json`
6. `fontmin.config.jsonc`

JSON and JSONC continue to be read and deserialized entirely in Rust. Module
configurations are evaluated by a short-lived Node.js child process and then
deserialized and executed by the Rust CLI.

A module may export its configuration as `default` or `config`. The exported
value may be a configuration object or a synchronous or asynchronous function
returning one. TypeScript support relies on the project's supported Node.js 22
or newer runtime and its built-in erasable TypeScript handling.

## Node Bridge

The CLI embeds a focused ESM bridge source string at compile time. For a module
configuration, Rust invokes:

```text
node --input-type=module --eval <embedded bridge> <absolute config path>
```

The bridge:

1. imports the configuration through a file URL;
2. selects `default` before the named `config` export;
3. awaits a function export when necessary;
4. validates the returned value recursively;
5. writes exactly one JSON document to stdout;
6. writes diagnostics to stderr and exits nonzero on failure.

The bridge is self-contained and does not depend on a script file beside the
Rust executable. Imports made by the user's configuration resolve normally
from the configuration's location, so a project-local `fontmin-rs` dependency
can provide `defineConfig()`, `modernWeb()`, and the other built-in presets.

## Serializable Boundary

The bridge accepts JSON-compatible configuration values plus the serializable
built-in plugin descriptors returned by the Node package. It rejects:

- custom plugin lifecycle or transform functions;
- a function-valued `css.fontFamily`;
- symbols and bigint values;
- cyclic objects;
- unknown plugin descriptors;
- built-in options that the Rust implementation cannot represent.

Errors identify the configuration path and the nearest field path, such as
`plugins[1].transform` or `css.fontFamily`. This avoids the silent property
loss that plain `JSON.stringify()` would cause.

## Built-in Plugin Representation

The Rust configuration model gains a serializable built-in plugin descriptor
matching the Node package's existing shape:

```json
{
  "name": "fontmin:ttf2woff2",
  "enforce": "post",
  "native": {
    "kind": "builtin",
    "name": "ttf2woff2",
    "options": { "clone": true, "quality": 11 }
  }
}
```

`enforce` is optional. The Rust pipeline constructs only known built-in
plugins and preserves their declaration order within each `pre`, normal, and
`post` group. It supports the descriptors produced by:

- `glyph`;
- `unicodeSlices`;
- `otf2ttf`;
- `ttf2woff`;
- `ttf2woff2`;
- `ttf2eot`;
- `ttf2svg`;
- `svg2ttf`;
- `svgs2ttf`;
- `css`;
- `modernWeb()`;
- `fontminCompatPreset()`.

Top-level `subset`, `delivery`, `outputs`, and `css` fields continue to create
their existing Rust plugins. Explicit `plugins` run first, matching the Node
package's `pluginsFromConfig()` behavior, and top-level output plugins follow.
Duplicate operations are therefore preserved rather than silently merged.

## CLI Overrides and Working Directory

The bridge returns configuration data only. Rust remains responsible for:

- assigning the configuration directory as `cwd` when `cwd` is absent;
- applying command-line input, output, subset, cache, preset, CSS, delivery,
  and variation overrides;
- input expansion, cache management, font processing, and output writes;
- all normal CLI reporting and diagnostics.

This keeps module and JSON configurations on the same execution path after
loading. Relative paths behave identically for every supported extension.

## Node Availability and Process Errors

Node.js is required only for module configuration files. If `node` cannot be
started, the CLI reports that module configs require Node.js 22 or newer and
suggests using JSONC when Node is unavailable. JSON and JSONC loading never
starts a child process.

A nonzero bridge exit includes bounded stderr in the Rust diagnostic. Empty
stdout, invalid JSON, or trailing non-whitespace output is reported as an
invalid bridge response. The CLI never treats console output as configuration
data; the bridge temporarily routes configuration-time `console` methods to
stderr so stdout remains machine-readable.

## Security and Execution Model

A module configuration is executable project code, just as it is in the Node
package. The CLI does not sandbox it. Documentation states that users should
only run trusted configuration modules. The child inherits the current
environment and working directory so normal module resolution and environment
lookups work, while the resolved configuration path is always passed as an
absolute path.

## Documentation

The English and Chinese configuration and CLI guides document:

- the shared discovery order;
- the Node.js requirement for module configs;
- async configuration factories;
- support for built-in presets;
- the serializable boundary and rejection of custom JavaScript hooks;
- JSONC as the dependency-free Rust CLI format.

`fontmin-rs init` continues to generate `fontmin.config.jsonc`.

## Verification

Tests must prove:

1. Explicit `.ts`, `.mts`, `.mjs`, and `.cjs` files load through the Rust CLI.
2. Default and named exports work, including an asynchronous factory.
3. Automatic discovery uses the same precedence as the Node package.
4. A module importing `modernWeb()` produces the expected Rust pipeline
   outputs.
5. CLI overrides apply after module evaluation.
6. Relative inputs, text files, cache paths, and output directories resolve
   from the module's directory.
7. Custom plugin functions, function-valued font families, unknown plugins,
   cyclic data, and unsupported built-in options fail with field-path
   diagnostics.
8. A missing Node executable yields the dedicated requirement message.
9. JSON and JSONC builds still work when Node is unavailable.
10. Existing Rust CLI integration tests and Node config-loading tests remain
    green.

No Node runtime is embedded, no command is delegated to the npm CLI, and no
release or package-version change is part of this work.
