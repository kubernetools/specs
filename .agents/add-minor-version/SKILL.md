# Skill: add-minor-version

Onboard a new Kubernetes minor-version series.

## When to use

When Kubernetes ships a new minor release (e.g. `v1.37.0`) and you want to start tracking it in this repository.

## Steps

### 1. Fetch specs for the new minor

Trigger the `fetch-specs` GitHub Action with the new minor version:

1. Go to **Actions → Fetch Kubernetes OpenAPI Specs**.
2. Click **Run workflow** and enter `1.37`.
3. Merge the PR it opens — this creates `specs/v1.37/` with `.version` and all JSON files.

### 2. Update AGENTS.md

Edit the "Supported versions" section in `AGENTS.md` to include the new minor, e.g.:

```
As of 2026-06 those are **v1.33 – v1.37**.
```

### 3. Open a pull request

```sh
git checkout -b specs/add-v1.37
git add specs/v1.37/ AGENTS.md
git commit -m "chore: add OpenAPI specs for v1.37"
git push origin specs/add-v1.37
gh pr create --title "chore: add OpenAPI specs for v1.37" --base main
```

## What changes

| Change | Path |
|---|---|
| New directory added | `specs/v1.37/` |
| Version list updated | `AGENTS.md` |

## Checks before merging

- `specs/v1.37/.version` exists and contains a `v1.37.*` tag.
- The number of JSON files is plausible (similar count to other minor versions).
