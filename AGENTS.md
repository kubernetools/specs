# AGENTS.md — kubernetools/specs

## Role of this repository

This is the spec cache for the [kubernetools](https://github.com/kubernetools/project) project. It stores Kubernetes OpenAPI v3 JSON files fetched from the upstream `kubernetes/kubernetes` repository and consumed by [`kubernetools/docgen`](https://github.com/kubernetools/docgen) to generate the documentation site.

Do not edit the JSON files under `specs/` by hand. They are fetched verbatim from upstream and must stay byte-for-byte identical to the source.

## Supported versions

Four minor-version series are tracked at any time, matching the [Kubernetes community support window](https://kubernetes.io/releases/patch-releases/#support-period) (N to N-3). As of 2026-06 those are **v1.33 – v1.36**.

## Repository layout

```
specs/                          cached OpenAPI v3 JSON, one dir per minor version
  v1.33/
    .version                    patch tag stored here, e.g. "v1.33.3"
    api__v1_openapi.json
    apis__apps__v1_openapi.json
    ...
  v1.34/ …

spec-fetcher/                   Rust binary that keeps specs up to date
  Cargo.toml
  src/main.rs

.github/workflows/
  fetch-specs.yml               manually-triggered Action (workflow_dispatch)

.agents/                        agent skill documentation
  fetch-specs/SKILL.md
  add-minor-version/SKILL.md
```

## spec-fetcher binary

The binary lives in `spec-fetcher/` and is built from source by the GitHub Action on each run.

**What it does**

1. Accepts a minor-version series (e.g. `1.33`) as a positional argument.
2. Queries the GitHub Releases API for `kubernetes/kubernetes` to find the highest stable patch in that series (e.g. `v1.33.3`).
3. Reads `specs/v1.33/.version` — if it already contains that tag, exits immediately (idempotent).
4. Downloads every `.json` file from `api/openapi-spec/v3/` at that tag via the GitHub Contents API and writes them flat into `specs/v1.33/`.
5. Writes the tag into `specs/v1.33/.version`.

**Build and test** (for development only — spec updates go through the Action)

```sh
cargo build --release --manifest-path spec-fetcher/Cargo.toml
```

**Test**

```sh
cargo test --manifest-path spec-fetcher/Cargo.toml
```

Tests use `wiremock` to intercept HTTP calls; no network access required.

## GitHub Action

`.github/workflows/fetch-specs.yml` is triggered manually (`workflow_dispatch`) with a single required input:

| Input | Description | Example |
|---|---|---|
| `minor_version` | Minor-version series to fetch | `1.33` |

The action builds the binary, runs it, and opens a pull request if the specs changed. It is a no-op when the stored version is already current.

## Constraints for agents

- **Never** commit directly to `main`. All changes go through a pull request.
- **Never** update `specs/` directly. All spec updates must go through the `fetch-specs` GitHub Action — it is the only permitted way to write to `specs/`.
- **Never** edit `specs/**/*.json` files by hand. They are fetched verbatim from upstream.
- The `spec-fetcher/` source is the only place where business logic lives. Keep it minimal.
- When adding a tracked minor version, update this file's "Supported versions" section.
