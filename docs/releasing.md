# Release preparation

Releases are prepared as a candidate first. The preparation commands do not
create a Git tag, push commits, publish npm packages, or dispatch the Release
workflow.

## Automated gate

Choose the next version once for the whole Rust/npm workspace:

```shell
pnpm run release:version
```

The command updates every published package, the Rust workspace, generated
binding guards, and Node/CLI version constants. It deliberately does not
commit, create a tag, or push. Review `CHANGELOG.md` separately and update
`Cargo.lock` if the Cargo check performed by the command changed it.

Install the tools used by CI, then run the complete release gate:

```shell
rustup component add llvm-tools-preview
cargo install cargo-deny cargo-llvm-cov
pnpm run release:check
```

The gate verifies:

- one non-placeholder version across the Rust workspace and all 11 published
  npm packages;
- a matching changelog heading;
- formatting, linting, typechecking, Rust/Node/WASM tests, and documentation;
- Rust advisory/source policy and high-severity npm advisories;
- a minimum 80% Rust line-coverage baseline (81.94% when established);
- dry-run package contents and installable Node/WASM/native package tarballs.

The advisory policy records two temporary maintenance exceptions:
`RUSTSEC-2024-0436` (`paste`, transitive through `woff2-patched`) and
`RUSTSEC-2026-0192` (`ttf-parser`). Neither advisory reports a vulnerability or
offers a safe upgrade. All vulnerability and unsoundness advisories still fail
the gate; remove each exception when its dependency path is replaced.

To validate a prospective tag without creating it:

```shell
pnpm run release:metadata -- --tag v<version>
```

Prerelease versions publish to the matching npm dist-tag (`beta`, `rc`, and so
on); stable versions publish to `latest`.

## Manual checklist

- [ ] `pnpm install --frozen-lockfile` succeeds from a clean checkout.
- [ ] `pnpm run release:check` passes locally.
- [ ] The main-branch CI matrix passes on Linux, macOS, and Windows.
- [ ] Node 22, 24, and 26 tests pass.
- [ ] Chromium, Firefox, and WebKit WASM tests pass.
- [ ] All eight native binding artifacts are present.
- [ ] `CHANGELOG.md` matches the candidate behavior and known limitations.
- [ ] The candidate version and prospective `v<version>` tag agree.
- [ ] Package tarballs contain no fixture, test, cache, or development files.

Stop after this checklist when preparing a release. Creating the tag and
running the Release workflow are separate, explicitly authorized actions.

## Routine release workflow

The Release workflow runs only when a `v*` tag is pushed. Push the version
commit to `main` and wait for the main-branch CI to pass before creating the
tag. Do not use `--follow-tags`, because that can start publishing before the
version commit has passed CI.

### 1. Start from a clean main branch

```shell
git switch main
git pull --ff-only
pnpm install --frozen-lockfile
git status --short
```

Do not prepare a release with unrelated working-tree changes.

### 2. Choose the version once

```shell
pnpm run release:version
```

Select the exact SemVer version in the `bumpp` prompt. For example, use
`0.1.0-beta.2` for the next beta or `0.1.0` for the stable release. This
updates all Rust and npm package manifests plus the embedded Node, CLI, and
native-binding version guards. It does not commit, tag, push, or publish.

Review any `Cargo.lock` update produced by the configured Cargo check. Then add
a dated `CHANGELOG.md` heading that exactly matches the selected version:

```markdown
## [0.1.0-beta.2] - YYYY-MM-DD
```

Also add or update the comparison link at the bottom of `CHANGELOG.md`.

### 3. Validate the candidate

Replace `<version>` with the version selected above, without angle brackets:

```shell
pnpm run release:metadata -- --tag v<version>
pnpm run release:check
git diff --check
git status --short
```

Both release commands must pass. Review the complete diff and confirm that all
11 npm packages, the Rust workspace, the changelog heading, and the prospective
tag use the same version.

### 4. Commit and validate main

From the clean release-only diff:

```shell
git add -u
git commit -m "chore(release): v<version>"
git push origin main
```

Wait for the main-branch CI matrix to finish successfully before continuing:

```shell
gh run list --branch main --limit 5
```

### 5. Create the release tag

Create an annotated tag on the exact commit that passed CI, then push only that
tag:

```shell
git tag -a "v<version>" -m "v<version>"
git push origin "v<version>"
```

The tag push starts `.github/workflows/release.yml`. The workflow verifies the
tag and metadata again, builds all eight native packages and the WASM package,
publishes all 11 npm packages through OIDC with provenance, and creates the
GitHub Release.

Prereleases use their channel as the npm dist-tag (`beta`, `rc`, and so on).
Numeric prerelease channels fall back to `next`; stable versions use `latest`.

### 6. Monitor and verify publication

```shell
gh run list --workflow Release --limit 5
gh run watch <run-id> --exit-status

npm view fontmin-rs@<version> version
npm view @fontmin-rs/binding@<version> version
npm view @fontmin-rs/wasm@<version> version
npm dist-tag ls fontmin-rs
gh release view "v<version>"
```

Verify the workflow is green, the expected dist-tag points to the new version,
the packages show provenance on npm, and the GitHub Release targets the same
tag.

### Failure recovery

- Never unpublish a released version or try to reuse its version number.
- If verification or native builds fail before npm publishing, fix the cause
  on `main`, run the complete release gate again, and publish a new version and
  tag. Do not move a pushed release tag.
- If publishing fails because of a transient registry error, first check which
  of the 11 versions exist on npm. Rerun only the failed job after confirming
  the existing packages are correct; recursive pnpm publishing skips versions
  already present in the registry.
- If the workflow itself needs a code change, make that change on `main` and use
  a new version and tag so the new workflow is part of the tagged commit.
- If npm publishing succeeds but GitHub Release creation fails, do not publish
  again. Create the GitHub Release for the existing tag, then fix the workflow
  for the next release.

## Trusted publishing

All 11 npm packages publish through GitHub Actions trusted publishing. The
Release workflow uses GitHub OIDC and does not require an npm access token.
Each package is configured with the `fontmin-rs/fontmin-rs` repository,
`release.yml` workflow, and publish permission.

To restore or audit a trusted publisher configuration, use npm 11.15 or newer
with account-level 2FA enabled:

```shell
npm trust github <package> \
  --repo fontmin-rs/fontmin-rs \
  --file release.yml \
  --allow-publish \
  --yes
```

The first command requests 2FA. npm can temporarily skip repeat 2FA checks so
multiple packages can be configured in the same five-minute window. Verify a
configuration with:

```shell
npm trust list <package> --json
```

Keep `permissions.id-token: write`, `registry-url`, and `--provenance` in the
workflow. The first publication was bootstrapped with a short-lived granular
access token because trusted publishing can only be configured after a package
exists.
