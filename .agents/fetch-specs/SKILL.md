# Skill: fetch-specs

Update the cached OpenAPI specs for a Kubernetes minor-version series.

## When to use

- A new patch release has shipped for a tracked minor (e.g. `v1.33.4` is out and `specs/v1.33/.version` still says `v1.33.3`).
- Specs for a minor version are missing entirely from `specs/`.
- You need to verify the stored specs match the latest upstream patch.

## Via GitHub Action

1. Go to **Actions → Fetch Kubernetes OpenAPI Specs** in the GitHub UI.
2. Click **Run workflow**.
3. Enter the minor version, e.g. `1.33`.
4. The action builds the binary, runs it, and opens a pull request if anything changed.
5. Review and merge the PR.

## What the fetcher writes

| Path | Content |
|---|---|
| `specs/v1.33/.version` | Exact patch tag fetched, e.g. `v1.33.3` |
| `specs/v1.33/*.json` | Flat OpenAPI v3 JSON files from `api/openapi-spec/v3/` at that tag |

## Expected output (up-to-date case)

```
Latest release for 1.33: v1.33.3
Already up to date (v1.33.3), nothing to do.
```

## Expected output (update case)

```
Latest release for 1.33: v1.33.4
Stored: v1.33.3, updating to v1.33.4
Downloading 47 spec files...
  wrote api__v1_openapi.json
  wrote apis__apps__v1_openapi.json
  ...
Done: stored v1.33.4 (47 files) in specs/v1.33
```

## Checks before merging the PR

- `specs/v1.33/.version` contains the new tag.
- The number of JSON files is plausible (Kubernetes v1.33 has ~47 spec files; a large drop or addition is worth investigating).
- No `.json` files were accidentally removed without being replaced.
