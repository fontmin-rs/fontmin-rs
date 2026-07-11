# Repository Guidelines

## Project Structure & Module Organization

This is a Rust workspace with Node.js packaging. Core Rust libraries live in `crates/`, with focused crates such as `fontmin_core`, `fontmin_subset`, and format-specific crates like `fontmin_woff2`. The CLI app is in `apps/fontmin`, and integration tests for it are in `apps/fontmin/tests`. Native Node bindings are in `napi/fontmin`, while the published TypeScript package is in `packages/fontmin`. Platform-specific npm binding manifests live under `npm/`. Shared fixtures, including test fonts, are in `fixtures/`, and design notes are in `docs/`.

## Build, Test, and Development Commands

- `pnpm install`: install workspace dependencies using pnpm 11.
- `pnpm run build`: build the TypeScript package through the workspace pipeline.
- `pnpm run build:debug`: build the local N-API binding for tests and development.
- `pnpm run test`: build debug bindings, run Vitest package tests, then run `cargo test --workspace`.
- `pnpm run check`: run format checks, linting, typechecking, and tests.
- `cargo run -p fontmin_app -- inspect fixtures/fonts/ttf/roboto-regular.ttf --json`: run the CLI locally.

Automated agents in this environment should prefix shell commands with `rtk`, except `pnpm typecheck`.

## Coding Style & Naming Conventions

Use Rust 2024 with the pinned toolchain in `rust-toolchain.toml`. Rust code is formatted with `cargo fmt --all` and linted with workspace Clippy settings, including `clippy::pedantic` warnings. TypeScript and JavaScript use ESM, strict TypeScript settings, `oxfmt`, and `oxlint`. Follow `.editorconfig`: UTF-8, LF endings, final newline, two-space indentation, and trimmed trailing whitespace except in Markdown. Use snake_case for Rust modules/functions and kebab-case or scoped npm package names for packages.

## Testing Guidelines

Rust tests use Cargo’s built-in test framework; place crate-level integration tests in `crates/<name>/tests` and CLI integration tests in `apps/fontmin/tests`. TypeScript tests use Vitest and follow the existing `*.test.ts` pattern in `packages/fontmin/test` and `napi/fontmin/test`. Prefer fixture-based tests using files in `fixtures/` for font behavior.

## Commit & Pull Request Guidelines

Git history follows Conventional Commits, for example `feat: expand glob inputs`, `ci: build native release artifacts`, and `fix: ...`. Keep commits scoped and imperative. Pull requests should describe the behavior change, list test commands run, link related issues when available, and include CLI output or screenshots only when they clarify user-visible behavior.
