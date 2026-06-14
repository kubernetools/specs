# Skill: pin-github-action

Add a GitHub Action to a workflow, pinned to a commit SHA.

## Convention

All actions are pinned to a **commit SHA**, never a floating tag. A comment on the same line records the human-readable version:

```yaml
- uses: actions/checkout@df4cb1c069e1874edd31b4311f1884172cec0e10  # v6.0.3
```

Dependabot (configured in `.github/dependabot.yml`) opens PRs to bump the SHA whenever a new release is published. Do not update SHAs by hand.

## How to pin a new action

### 1. Find the latest release tag

```sh
gh api repos/<owner>/<action>/releases/latest --jq '.tag_name'
# e.g. "v6.0.3"
```

### 2. Resolve the tag to a commit SHA

Tags can be lightweight (point directly to a commit) or annotated (point to a tag object that wraps a commit). Always resolve to the underlying commit SHA.

```sh
# Step 1 — get the object the tag points to
OBJ=$(gh api repos/<owner>/<action>/git/ref/tags/<tag> --jq '.object | {sha, type}')
SHA=$(echo "$OBJ" | jq -r '.sha')
TYPE=$(echo "$OBJ" | jq -r '.type')

# Step 2 — if it's an annotated tag object, dereference once more
if [ "$TYPE" = "tag" ]; then
  SHA=$(gh api repos/<owner>/<action>/git/tags/$SHA --jq '.object.sha')
fi

echo "$SHA"
```

### 3. Add to the workflow

```yaml
- uses: <owner>/<action>@<commit-sha>  # <tag>
```

Example:

```yaml
- uses: actions/checkout@df4cb1c069e1874edd31b4311f1884172cec0e10  # v6.0.3
```

## Dependabot config

`.github/dependabot.yml` enables weekly automated updates for all actions in this repository:

```yaml
version: 2
updates:
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
```

When Dependabot opens a PR it will update both the SHA and the version comment. Review and merge as normal.
