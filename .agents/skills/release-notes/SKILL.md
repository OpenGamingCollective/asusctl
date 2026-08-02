---
name: release-notes
description: >
  Release lifecycle and changelog drafting skill for asusctl. Summarizes
  commit messages, PR descriptions, and touched files into flat
  CHANGELOG.md entries with PR links (#NN) and contributor thanks (@handle).
  Bumps workspace package versioning in Cargo.toml, updates CHANGELOG, and
  creates release commits/tags. Excludes breaking change analysis and gh
  release create execution. Trigger when drafting release notes, preparing
  a new version release, or updating CHANGELOG.
---

# asusctl release-notes

Executes the release workflow: draft notes → decide version → bump Cargo.toml + prepend CHANGELOG → commit & tag → push.

## DO NOTs & Critical Constraints

- **NEVER** run `gh release create`. Always print the exact command for the maintainer at the very end.
- **NEVER** request the `commits` field in `gh pr list` (causes a GitHub GraphQL 500k node limit crash).
- **NEVER** run per-PR loops or attempt commit-to-PR SHA/subject matching (rebase-merge rewrites SHAs; `gh` truncates message headlines).
- **NEVER** thank the maintainer cutting the release. Only PR authors(if they authored a PR themselves, they can tag themselves) (`Thanks @handle`) and co-authors listed in PR bodies (`Credits to @handle`).
- **NEVER** credit bots (`dependabot`, `@app/*`).
- **NEVER** copy raw commit subjects verbatim — rewrite internal mechanisms into user-visible outcomes.
- **NEVER** proceed if `HEAD..origin/main` is not empty — STOP immediately and ask the maintainer to sync (`git pull` / `git rebase`).

## Step 1 — Boundary & Sync Check

```bash
git fetch origin main
LAST_TAG=$(git tag --sort=-creatordate | head -1) && echo $LAST_TAG
git --no-pager log HEAD..origin/main --oneline
```
*If `HEAD..origin/main` output is non-empty, STOP and ask the maintainer to sync first.*

## Step 2 — Fetch In-Range PRs (Single Call)

```bash
TAG_DATE=$(git log -1 --format=%aI "$LAST_TAG") && echo "$TAG_DATE"
gh pr list --repo OpenGamingCollective/asusctl \
  --state merged \
  --search "merged:>$TAG_DATE" \
  --json number,title,author,body,files,baseRefName
```
*Filter: Keep only PRs where `baseRefName == "main"`.*

Optional direct commit inventory check:
```bash
git --no-pager log --no-merges "${LAST_TAG}..HEAD" --format='%h|%ae|%s'
```

## Step 3 — Draft CHANGELOG Entry

- **Format:** `- <User-visible change>: Thanks @handle (#NN)`.
- **Co-authors:** Scan PR `body` for `Credits to @handle` or `thanks @handle` lines.
- **Rollup:** Consolidate pure CI, packaging, test, or refactor changes into 1 summary line (e.g. `Packaging, CI/CD and Issue tracking optimizations: Thanks @a and @b (#228)`).
- **Noise:** Ignore `Release: X.Y.Z` and `chore: update CHANGELOG` commits.
- **Upgrade Notes:** Always Ask the maintainer if existing users need manual action (e.g. post-upgrade autostart file cleanup). If yes, append `### Notes` explaining: **Who is affected / What happens / Action required**.

## Step 4 — Version Selection

Present a suggestion box(use tool if exists):
1. Next Major (X+1.Y.Z)
2. Next Minor (X.Y+1.Z)
3. Next Patch (X.Y.Z+1)
4. Type your own

Guidance: New features / device support → Minor; fixes / chores only → Patch.

## Step 5 — Apply Release (Maintainer Approved)

1. Set `version = "X.Y.Z"` in `Cargo.toml` (`[workspace.package] version`).
2. Prepend `## X.Y.Z` (and optional `### Notes`) to `CHANGELOG.md`.
3. Commit, tag, and push:
```bash
git add Cargo.toml CHANGELOG.md
git commit -m "Release: X.Y.Z"
git tag X.Y.Z
git push origin main && git push origin X.Y.Z
```

## Step 6 — Instruct Maintainer for GitHub Release

Print this exact command for the maintainer to execute manually:

```bash
gh release create X.Y.Z \
  --repo OpenGamingCollective/asusctl \
  --verify-tag \
  --title X.Y.Z
```