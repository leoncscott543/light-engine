# Contributing to Light Engine

## Scope and tracking

Use GitHub issues for concrete bugs, features, and research milestones. Include
the observed or proposed behavior, scope, acceptance criteria, and validation
needed. Distinguish planned physics and infrastructure from implemented behavior.
Search existing issues and pull requests before opening duplicates.

## Branches and commits

- Start a focused `feat/`, `fix/`, `docs/`, or `chore/` branch from current `main`.
- Review the diff and stage explicit paths, never local credentials or build output.
- Commit coherent, verified increments with a descriptive subject and a body
  explaining significant behavior changes and validation.
- Push the branch normally; do not rewrite shared history.
- Keep machine-local environment files untracked. Use a sanitized `.env.example`
  only when configuration is actually needed.

## Validation

Run the workspace commands in [README.md](README.md). Add regression tests for
behavior changes. Record checks that could not run and why; do not claim native
OS, architecture, Docker, or numerical validation from unrelated checks.

Rust modules should explain their purpose with module-level documentation.
Introduce only usable workspace members, not empty future crate directories.

## Pull requests and completion

Describe the change, motivation, linked issues, tests and known limitations.
Use `Closes #<number>` only when the PR satisfies that issue's acceptance criteria.
Keep incomplete work as a draft. Do not merge with failing required checks or
unresolved review findings. Close superseded dependency PRs with an explanation,
not by merging obsolete manifests.

Delete merged branches only after confirming they contain no unmerged work.
Roadmap ideas are not automatically approval to implement or deploy them.
