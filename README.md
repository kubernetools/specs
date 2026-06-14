# specs

Cached Kubernetes OpenAPI v3 specs, one directory per supported minor version. Updated automatically via the [`fetch-specs`](.github/workflows/fetch-specs.yml) GitHub Action.

## Repository layout

```
specs/
  v1.33/
    .version                      ← patch tag currently stored, e.g. "v1.33.3"
    api__v1_openapi.json
    apis__apps__v1_openapi.json
    apis__batch__v1_openapi.json
    ...
  v1.34/
    .version
    ...
```

Each directory corresponds to a minor-version series. The `.version` file records the exact patch release the JSON files were fetched from. The JSON files are the flat OpenAPI v3 spec files from `api/openapi-spec/v3/` in the upstream `kubernetes/kubernetes` repository.

## Developing the fetcher locally

> Specs in this repository are updated exclusively through the `fetch-specs` GitHub Action. Local runs below are for development and testing of the binary only — do not commit spec changes produced locally.

### Prerequisites

- Rust toolchain (`rustup` or equivalent)
- A GitHub personal access token with **public repo read** scope (needed to raise the API rate limit from 60 to 5 000 req/hr; without it small fetches still work)

### Build

```sh
cargo build --release --manifest-path spec-fetcher/Cargo.toml
```

The binary is written to `spec-fetcher/target/release/spec-fetcher`.

### Run

```sh
GITHUB_TOKEN=<your-token> ./spec-fetcher/target/release/spec-fetcher --specs-dir /tmp/k8s-specs 1.33
```

Use `--specs-dir` to point at a temporary directory so the working tree stays clean. Run from the repository root.

### What the binary does

1. Queries the GitHub Releases API for `kubernetes/kubernetes` to find the latest stable patch in the `v1.33.*` series.
2. Reads `<specs-dir>/v1.33/.version` — if it already contains that tag, exits immediately.
3. Downloads every `.json` file from `api/openapi-spec/v3/` at that tag and writes them flat into `<specs-dir>/v1.33/`.
4. Writes the tag into `<specs-dir>/v1.33/.version`.

### CLI reference

```
Fetch Kubernetes OpenAPI v3 specs for a given minor version series

Usage: spec-fetcher [OPTIONS] <MINOR_VERSION>

Arguments:
  <MINOR_VERSION>
          Kubernetes minor version to fetch, e.g. "1.33"

Options:
      --specs-dir <SPECS_DIR>
          Directory to store specs

          [default: ./specs]

      --github-token <GITHUB_TOKEN>
          GitHub API token (or GITHUB_TOKEN env var)

          [env: GITHUB_TOKEN=]

  -h, --help
          Print help
```

### Running the tests

```sh
cargo test --manifest-path spec-fetcher/Cargo.toml
```
