# Build Pages Workflow Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the VitePress documentation on every push to `main` and publish the static output to the `pages` branch for `fontmin-rs.ntnyq.dev`.

**Architecture:** A standalone GitHub Actions workflow installs the repository's existing Node, pnpm, Rust, and wasm-pack toolchain, generates the WASM bindings consumed by the docs playground, and builds VitePress. A final `peaceiris/actions-gh-pages@v4` step replaces the `pages` branch contents with `docs/.vitepress/dist` and writes the custom-domain CNAME.

**Tech Stack:** GitHub Actions, pnpm 11, Node.js LTS, Rust wasm32 target, wasm-pack, VitePress, Vitest, `peaceiris/actions-gh-pages@v4`

## Global Constraints

- Trigger deployment only for pushes to `main`.
- Publish generated static files to the `pages` branch.
- Serve from the custom-domain root `fontmin-rs.ntnyq.dev`; keep the VitePress base path at `/`.
- Generate WASM bindings before building docs because the playground imports them.
- Grant only `contents: write` to the workflow token.
- Cancel an older in-progress Pages deployment when a newer `main` push arrives.

---

### Task 1: Add and verify the Pages deployment workflow

**Files:**
- Create: `.github/workflows/build-pages.yml`
- Modify: `packages/fontmin/tests/package.test.ts`

**Interfaces:**
- Consumes: `pnpm -C wasm/fontmin run build:wasm`, `pnpm run docs:build`, and the generated `docs/.vitepress/dist` directory.
- Produces: A `pages` branch whose root contains the VitePress build and a `CNAME` file containing `fontmin-rs.ntnyq.dev`.

- [ ] **Step 1: Write the failing workflow configuration test**

Add this test after `defines repository ci gates` in `packages/fontmin/tests/package.test.ts`:

```ts
it('publishes documentation to the pages branch on main pushes', () => {
  const workflowPath = resolve(
    repositoryRoot,
    '.github/workflows/build-pages.yml',
  )

  expect(existsSync(workflowPath)).toBe(true)

  const workflow = readFileSync(workflowPath, 'utf8')

  expect(workflow).toContain('name: Build Pages')
  expect(workflow).toContain('branches:\n      - main')
  expect(workflow).toContain('contents: write')
  expect(workflow).toContain('cancel-in-progress: true')
  expect(workflow).toContain('targets: wasm32-unknown-unknown')
  expect(workflow).toContain('jetli/wasm-pack-action@v0.4.0')
  expect(workflow).toContain('pnpm install --frozen-lockfile')
  expect(workflow).toContain('pnpm -C wasm/fontmin run build:wasm')
  expect(workflow).toContain('pnpm run docs:build')
  expect(workflow).toContain('peaceiris/actions-gh-pages@v4')
  expect(workflow).toContain('publish_branch: pages')
  expect(workflow).toContain('publish_dir: ./docs/.vitepress/dist')
  expect(workflow).toContain('cname: fontmin-rs.ntnyq.dev')
})
```

- [ ] **Step 2: Run the test and verify the missing workflow fails**

Run:

```bash
rtk pnpm exec vitest --run packages/fontmin/tests/package.test.ts
```

Expected: FAIL in `publishes documentation to the pages branch on main pushes` because `.github/workflows/build-pages.yml` does not exist.

- [ ] **Step 3: Add the minimal deployment workflow**

Create `.github/workflows/build-pages.yml`:

```yaml
name: Build Pages

on:
  push:
    branches:
      - main

permissions:
  contents: write

concurrency:
  group: build-pages
  cancel-in-progress: true

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: pnpm/action-setup@v6
      - uses: actions/setup-node@v6
        with:
          node-version: lts/*
          cache: pnpm
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown
      - uses: jetli/wasm-pack-action@v0.4.0

      - run: pnpm install --frozen-lockfile
      - run: pnpm -C wasm/fontmin run build:wasm
      - run: pnpm run docs:build
      - uses: peaceiris/actions-gh-pages@v4
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_branch: pages
          publish_dir: ./docs/.vitepress/dist
          cname: fontmin-rs.ntnyq.dev
```

- [ ] **Step 4: Run targeted tests and local documentation build**

Run:

```bash
rtk pnpm exec vitest --run packages/fontmin/tests/package.test.ts
rtk pnpm -C wasm/fontmin run build:wasm
rtk pnpm run docs:check
```

Expected: package tests pass, wasm-pack exits 0, all docs tests pass, and VitePress reports a successful build.

- [ ] **Step 5: Run repository validation**

Run:

```bash
rtk pnpm run check
rtk git diff --check
```

Expected: both commands exit 0. Existing non-fatal Clippy warnings may still be printed.

- [ ] **Step 6: Commit the implementation**

```bash
rtk git add .github/workflows/build-pages.yml packages/fontmin/tests/package.test.ts docs/superpowers/plans/2026-07-11-build-pages-workflow.md
rtk git commit -m "ci: publish documentation to pages"
```
