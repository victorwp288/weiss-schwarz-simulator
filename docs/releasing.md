# Releasing

This repo uses Release Please for automated release PRs and GitHub Releases. This document is intentionally practical and local-first: run all gates locally, then push/merge when ready.

## Local preflight (required)

From the repo root:

- `SKIP_BENCHMARKS=1 ./scripts/run_local_ci_parity.sh`
- `./scripts/freeze_preflight_235.sh /tmp/wss_freeze_candidate_v0_5_0`

If either script fails, fix locally and re-run until green.

## Version alignment

For a release at `0.6.0`, keep these in sync:

- `pyproject.toml` (`[project].version`)
- `weiss_core/Cargo.toml` (`[package].version`)
- `weiss_py/Cargo.toml` (`[package].version`)
- `.release-please-manifest.json` (`"."`)

## Release Please flow (remote steps)

Release Please opens a PR from `main` when it detects conventional commits since the last release.

Suggested workflow:

1) Push a branch with your changes and open a PR into `main`.
2) Merge once CI is green.
3) Let Release Please open its release PR.
4) Review the changelog/version bumps and merge the release PR.
5) Verify workflows ran for the tag (notably wheels).

## Wheels publish verification

After the release tag exists:

1) Verify `.github/workflows/wheels.yml` ran for the tag.
2) Confirm artifacts were uploaded/published as expected (wheels + sdist).
3) If a workflow didn’t trigger automatically, re-run/dispatch it for the release tag.
