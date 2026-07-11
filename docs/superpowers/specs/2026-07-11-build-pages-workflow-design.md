# Build Pages Workflow Design

## Goal

Publish the VitePress documentation site to the `pages` branch whenever a
commit is pushed to `main`. The published site uses the custom domain
`fontmin-rs.ntnyq.dev`.

## Workflow

Add `.github/workflows/build-pages.yml` with a single deployment job:

1. Trigger only on pushes to `main`.
2. Grant `contents: write` so the workflow can update the `pages` branch.
3. Use a concurrency group that cancels an older in-progress Pages deployment.
4. Check out the repository and install pnpm, Node.js, the Rust WASM target, and
   wasm-pack.
5. Install workspace dependencies with the frozen lockfile.
6. Generate the WASM bindings required by the documentation playground.
7. Build the VitePress site from `docs`.
8. Publish `docs/.vitepress/dist` to the `pages` branch with
   `peaceiris/actions-gh-pages`.
9. Write `fontmin-rs.ntnyq.dev` as the deployed branch's `CNAME` value.

The deployment replaces the generated branch contents on each run. Source
files and build dependencies remain on `main`; only static output is committed
to `pages`.

## Site Configuration

Keep the VitePress base path at `/` because the site is served from the custom
domain root rather than from the repository path.

## Failure Handling

The deploy step runs only after dependency installation, WASM generation, and
the VitePress build succeed. GitHub Actions reports failures without changing
the existing `pages` branch. Concurrency prevents an older build from
overwriting a newer push.

## Verification

Add repository-level configuration assertions for the workflow trigger,
permissions, build commands, publish directory, target branch, and custom
domain. Run those tests, build the docs locally, and run workflow/configuration
format checks before committing the implementation.
