# Repository Guidelines

## Project Structure & Module Organization

This monorepo combines a Rust workspace with Node.js packaging. Core libraries live in `crates/`, including `fontmin_core`, `fontmin_subset`, and format-specific crates such as `fontmin_woff2`. The CLI is in `apps/fontmin`, with integration tests under `apps/fontmin/tests`. Native Node bindings live in `napi/fontmin`; the published TypeScript package is in `packages/fontmin`. WASM sources are under `wasm/`, platform-specific npm manifests under `npm/`, shared font fixtures under `fixtures/`, and documentation under `docs/`.

## Build, Test, and Development Commands

- `pnpm install`: install the pnpm 11 workspace dependencies.
- `pnpm run build`: build the published TypeScript package.
- `pnpm run build:debug`: compile local debug N-API bindings.
- `pnpm run test`: build bindings and packages, then run WASM, Vitest, Cargo, and tooling tests.
- `pnpm run check`: run formatting, linting, type checks, tests, and documentation checks.
- `pnpm run coverage:check`: enforce at least 80% Rust line coverage.
- `cargo run -p fontmin_app -- inspect fixtures/fonts/ttf/roboto-regular.ttf --json`: run the CLI locally.

Automated agents in this environment must prefix shell commands with `rtk`, except `pnpm typecheck`.

## Coding Style & Naming Conventions

Use Rust 2024 with the toolchain pinned by `rust-toolchain.toml`. Format Rust with `cargo fmt --all` and lint with workspace Clippy settings, including `clippy::pedantic`. TypeScript and JavaScript use ESM, strict TypeScript, `oxfmt`, and `oxlint`. Follow `.editorconfig`: UTF-8, LF endings, final newlines, two-space indentation, and no trailing whitespace except in Markdown. Use `snake_case` for Rust modules and functions; use kebab-case or scoped names for npm packages.

## Testing Guidelines

Rust uses Cargo’s built-in test framework. Put crate integration tests in `crates/<name>/tests` and CLI tests in `apps/fontmin/tests`. TypeScript tests use Vitest and the `*.test.ts` convention in locations such as `packages/fontmin/test` and `napi/fontmin/test`. Prefer reusable files from `fixtures/` for font behavior. Run targeted tests while developing and `pnpm run check` before submitting.

## Commit & Pull Request Guidelines

Follow the Conventional Commits style visible in history, such as `feat: add optimize runtime selection`, `fix(docs): restore playground styles`, or `ci: build native artifacts`. Keep commits focused and imperative. Pull requests should explain the behavior change, list verification commands, and link relevant issues. Add CLI output or screenshots only when they clarify user-visible behavior.
